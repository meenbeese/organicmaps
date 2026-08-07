#include "dev_sandbox/cshim/c_api.h"

#include "map/framework.hpp"
#include "platform/location.hpp"
#include "platform/platform.hpp"
#include "platform/settings.hpp"

#include "base/assert.hpp"
#include "base/logging.hpp"
#include "base/task_loop.hpp"

#include "drape/drape_global.hpp"
#include "drape/glsl_types.hpp"
#include "drape/graphics_context.hpp"
#include "drape/graphics_context_factory.hpp"
#include "drape/mesh_object.hpp"
#include "drape/pointers.hpp"
#include "drape/render_state.hpp"
#include "drape/static_texture.hpp"
#include "drape/texture_manager.hpp"

#include "drape_frontend/drape_engine.hpp"
#include "drape_frontend/gui/skin.hpp"
#include "drape_frontend/render_state_extension.hpp"
#include "drape_frontend/user_event_stream.hpp"
#include "shaders/program_manager.hpp"
#include "storage/storage.hpp"

#include <algorithm>
#include <array>
#include <cstring>
#include <limits>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

struct GLFWwindow;

namespace
{
// ---------------------------------------------------------------------------
// GUI task loop. Drains queued tasks on the thread that calls Execute.
// ---------------------------------------------------------------------------
class SandboxTaskLoop : public base::TaskLoop
{
public:
  PushResult Push(Task && task) override
  {
    std::lock_guard<std::mutex> lock(m_mutex);
    m_tasks.emplace_back(std::move(task));
    return {true, base::TaskLoop::kNoId};
  }

  PushResult Push(Task const & task) override
  {
    std::lock_guard<std::mutex> lock(m_mutex);
    m_tasks.emplace_back(task);
    return {true, base::TaskLoop::kNoId};
  }

  void ExecuteTasks()
  {
    std::lock_guard<std::mutex> lock(m_mutex);
    for (auto & task : m_tasks)
      task();
    m_tasks.clear();
  }

private:
  std::vector<Task> m_tasks;
  std::mutex m_mutex;
};

// ---------------------------------------------------------------------------
// Framework + callbacks + gui skin.
// ---------------------------------------------------------------------------
struct SandboxFramework
{
  explicit SandboxFramework(bool enableDiffs) : framework(FrameworkParams(enableDiffs)) {}

  Framework framework;
  gui::Skin * skin = nullptr;

  void * user = nullptr;
  OmCountryChangedFn onCountryChanged = nullptr;
  OmDownloadProgressFn onDownloadProgress = nullptr;
  OmRenderInjectionFn onRenderInjection = nullptr;

  int subscribeId = 0;
};

// ---------------------------------------------------------------------------
// ImGui -> drape backend. The UI runs in Rust (imgui-rs); each frame it pushes
// its draw data and font atlas here; this renders them with a drape mesh object
// on the render thread via the om_fw_set_callbacks render injection.
// ---------------------------------------------------------------------------
class ImguiRenderer
{
public:
  ImguiRenderer() : m_state(df::CreateRenderState(gpu::Program::ImGui, df::DepthLayer::GuiLayer))
  {
    m_state.SetDepthTestEnabled(false);
    m_state.SetBlending(dp::Blending(true));
  }

  void Render(ref_ptr<dp::GraphicsContext> context, ref_ptr<dp::TextureManager> textureManager,
              ref_ptr<gpu::ProgramManager> programManager)
  {
    std::lock_guard<std::mutex> lock(m_bufferMutex);
    size_t renderDataIndex = (m_updateIndex + 1) % m_uiDataBuffer.size();
    UiDataBuffer & dataBuffer = m_uiDataBuffer[renderDataIndex];

    auto gpuProgram = programManager->GetProgram(m_state.GetProgram<gpu::Program>());

    bool needUpdate = true;
    if (!m_mesh || dataBuffer.m_vertices.size() > m_vertexCount || dataBuffer.m_indices.size() > m_indexCount)
    {
      while (dataBuffer.m_vertices.size() > m_vertexCount)
        m_vertexCount *= 2;
      while (dataBuffer.m_indices.size() > m_indexCount)
        m_indexCount *= 2;
      m_indexCount = std::min(m_indexCount, static_cast<uint32_t>(std::numeric_limits<uint16_t>::max()));

      dataBuffer.m_vertices.resize(m_vertexCount);
      dataBuffer.m_indices.resize(m_indexCount);

      m_mesh = make_unique_dp<dp::MeshObject>(context, dp::MeshObject::DrawPrimitive::Triangles, "imGui");

      m_mesh->SetBuffer(0, std::move(dataBuffer.m_vertices));
      m_mesh->SetAttribute("a_position", 0, 0 /* offset */, 2);
      m_mesh->SetAttribute("a_texCoords", 0, 2 * sizeof(float) /* offset */, 2);
      m_mesh->SetAttribute("a_color", 0, 4 * sizeof(float) /* offset */, 4);
      m_mesh->SetIndexBuffer(std::move(dataBuffer.m_indices));
      m_mesh->Build(context, gpuProgram);

      dataBuffer.m_vertices.clear();
      dataBuffer.m_indices.clear();
      needUpdate = false;
    }

    if (!m_texture)
    {
      std::lock_guard<std::mutex> lock(m_textureMutex);
      if (!m_textureData.empty())
      {
        m_texture = make_unique_dp<dp::StaticTexture>();
        m_texture->Create(context,
                          dp::Texture::Params{
                              .m_width = m_textureWidth,
                              .m_height = m_textureHeight,
                              .m_format = dp::TextureFormat::RGBA8,
                              .m_allocator = textureManager->GetTextureAllocator(),
                          },
                          m_textureData.data());
        m_textureData.clear();
        m_state.SetColorTexture(make_ref(m_texture));
      }
      else
      {
        // Can't render without texture.
        return;
      }
    }

    if (dataBuffer.m_drawCalls.empty())
      return;

    if (needUpdate && !dataBuffer.m_vertices.empty() && !dataBuffer.m_indices.empty())
    {
      m_mesh->UpdateBuffer(context, 0, dataBuffer.m_vertices);
      m_mesh->UpdateIndexBuffer(context, dataBuffer.m_indices);
      dataBuffer.m_vertices.clear();
      dataBuffer.m_indices.clear();
    }

    gpu::ImGuiProgramParams const params{.m_projection = m_projection};
    context->PushDebugLabel("ImGui Rendering");
    m_mesh->Render(context, gpuProgram, m_state, programManager->GetParamsSetter(), params, [&, this]()
    {
      context->SetCullingEnabled(false);
      for (auto const & drawCall : dataBuffer.m_drawCalls)
      {
        uint32_t y = drawCall.clipRect.y;
        if (context->GetApiVersion() == dp::ApiVersion::OpenGLES3)
          y = dataBuffer.m_height - y - drawCall.clipRect.w;
        context->SetScissor(drawCall.clipRect.x, y, drawCall.clipRect.z, drawCall.clipRect.w);
        m_mesh->DrawPrimitivesSubsetIndexed(context, drawCall.indexCount, drawCall.startIndex);
      }
      context->SetCullingEnabled(true);
      context->SetScissor(0, 0, dataBuffer.m_width, dataBuffer.m_height);
    });
    context->PopDebugLabel();
  }

  // Consumes one frame of imgui-rs draw data produced on the GUI thread.
  void Update(OmImGuiDrawList const * lists, uint32_t listCount, float displayPosX, float displayPosY,
              float displaySizeX, float displaySizeY, float framebufferScaleX, float framebufferScaleY)
  {
    UiDataBuffer & dataBuffer = m_uiDataBuffer[m_updateIndex];
    dataBuffer.m_drawCalls.clear();

    auto const fbWidth = static_cast<int>(displaySizeX * framebufferScaleX);
    auto const fbHeight = static_cast<int>(displaySizeY * framebufferScaleY);
    if (fbWidth <= 0 || fbHeight <= 0 || listCount == 0)
      return;
    dataBuffer.m_width = static_cast<uint32_t>(fbWidth);
    dataBuffer.m_height = static_cast<uint32_t>(fbHeight);

    size_t totalVertexCount = 0;
    size_t totalIndexCount = 0;
    int totalDrawCallsCount = 0;
    for (uint32_t i = 0; i < listCount; ++i)
    {
      totalVertexCount += lists[i].vertexCount;
      totalIndexCount += lists[i].indexCount;
      totalDrawCallsCount += lists[i].cmdCount;
    }

    CHECK(totalVertexCount <= std::numeric_limits<uint16_t>::max(),
          ("UI is so complex and now requires 32-bit indices. You need to improve dp::MeshObject or simplify UI"));
    CHECK(totalIndexCount <= std::numeric_limits<uint16_t>::max(), ());

    dataBuffer.m_vertices.resize(totalVertexCount);
    dataBuffer.m_indices.resize(totalIndexCount);
    dataBuffer.m_drawCalls.reserve(totalDrawCallsCount);

    float const clipOffX = displayPosX;
    float const clipOffY = displayPosY;
    float const clipScaleX = framebufferScaleX;
    float const clipScaleY = framebufferScaleY;

    uint32_t vertexOffset = 0;
    uint32_t indexOffset = 0;
    for (uint32_t i = 0; i < listCount; ++i)
    {
      OmImGuiDrawList const & list = lists[i];
      for (uint32_t j = 0; j < list.vertexCount; ++j)
      {
        OmImGuiVertex const & v = list.vertices[j];
        dp::Color color(v.color);
        dataBuffer.m_vertices[vertexOffset + j] = {
            .position = {v.x, v.y},
            .texCoords = {v.u, v.v},
            .color = {color.GetAlphaF(), color.GetBlueF(), color.GetGreenF(),
                      color.GetRedF()}  // Byte order is reversed in imGui
        };
      }

      for (uint32_t j = 0; j < list.indexCount; ++j)
      {
        uint32_t indexValue = list.indices[j];
        indexValue += vertexOffset;
        CHECK(indexValue <= std::numeric_limits<uint16_t>::max(), ());
        dataBuffer.m_indices[indexOffset + j] = static_cast<uint16_t>(indexValue);
      }

      for (uint32_t cmdIndex = 0; cmdIndex < list.cmdCount; ++cmdIndex)
      {
        OmImGuiCmd const & cmd = list.cmds[cmdIndex];
        float clipMinX = (cmd.clipX - clipOffX) * clipScaleX;
        float clipMinY = (cmd.clipY - clipOffY) * clipScaleY;
        float clipMaxX = (cmd.clipZ - clipOffX) * clipScaleX;
        float clipMaxY = (cmd.clipW - clipOffY) * clipScaleY;
        if (clipMinX < 0.0f)
          clipMinX = 0.0f;
        if (clipMinY < 0.0f)
          clipMinY = 0.0f;
        if (clipMaxX > fbWidth)
          clipMaxX = static_cast<float>(fbWidth);
        if (clipMaxY > fbHeight)
          clipMaxY = static_cast<float>(fbHeight);
        if (clipMaxX <= clipMinX || clipMaxY <= clipMinY)
          continue;

        dataBuffer.m_drawCalls.emplace_back(DrawCall{
            .indexCount = cmd.elemCount,
            .startIndex = static_cast<uint32_t>(indexOffset + cmd.idxOffset),
            .clipRect = {static_cast<uint32_t>(clipMinX), static_cast<uint32_t>(clipMinY),
                         static_cast<uint32_t>(clipMaxX - clipMinX), static_cast<uint32_t>(clipMaxY - clipMinY)}});
      }

      vertexOffset += list.vertexCount;
      indexOffset += list.indexCount;
    }
    CHECK(vertexOffset == totalVertexCount, ());
    CHECK(indexOffset == totalIndexCount, ());

    {
      std::lock_guard<std::mutex> lock(m_bufferMutex);

      // Orthographic projection (transposed for `vec * mat` shader convention).
      float const left = displayPosX;
      float const right = displayPosX + displaySizeX;
      float const top = displayPosY;
      float const bottom = displayPosY + displaySizeY;
      m_projection = glsl::mat4(2.0f / (right - left), 0.0f, 0.0f, -(right + left) / (right - left),  // col 0
                                0.0f, 2.0f / (top - bottom), 0.0f, -(top + bottom) / (top - bottom),  // col 1
                                0.0f, 0.0f, -1.0f, 0.0f,                                              // col 2
                                0.0f, 0.0f, 0.0f, 1.0f);                                              // col 3

      // Swap buffers
      m_updateIndex = (m_updateIndex + 1) % m_uiDataBuffer.size();
    }
  }

  // Uploads the font atlas texture (built by imgui-rs) for the render thread.
  void SetTexture(uint32_t width, uint32_t height, uint8_t const * rgba, size_t len)
  {
    std::lock_guard<std::mutex> lock(m_textureMutex);
    m_textureData.assign(rgba, rgba + len);
    m_textureWidth = width;
    m_textureHeight = height;
  }

  void Reset()
  {
    {
      std::lock_guard<std::mutex> lock(m_textureMutex);
      m_texture.reset();
    }

    {
      std::lock_guard<std::mutex> lock(m_bufferMutex);
      m_mesh.reset();
    }
  }

private:
  struct ImguiVertex
  {
    glsl::vec2 position;
    glsl::vec2 texCoords;
    glsl::vec4 color;
  };
  static_assert(sizeof(ImguiVertex) == 2 * sizeof(glsl::vec4));

  struct DrawCall
  {
    uint32_t indexCount = 0;
    uint32_t startIndex = 0;
    glsl::uvec4 clipRect{};
  };

  drape_ptr<dp::MeshObject> m_mesh;
  uint32_t m_vertexCount = 2000;
  uint32_t m_indexCount = 3000;

  drape_ptr<dp::StaticTexture> m_texture;
  std::vector<unsigned char> m_textureData;
  uint32_t m_textureWidth = 0;
  uint32_t m_textureHeight = 0;

  dp::RenderState m_state;

  struct UiDataBuffer
  {
    std::vector<ImguiVertex> m_vertices;
    std::vector<uint16_t> m_indices;
    std::vector<DrawCall> m_drawCalls;
    uint32_t m_width;
    uint32_t m_height;
  };
  std::array<UiDataBuffer, 2> m_uiDataBuffer;
  size_t m_updateIndex = 0;

  glsl::mat4 m_projection{0.0f};

  std::mutex m_bufferMutex;
  std::mutex m_textureMutex;
};
}  // namespace

// Per-platform graphics context factory entry points (context_*.cpp / .mm).
// Defined at global scope in the platform files; declared here for the shim.
drape_ptr<dp::GraphicsContextFactory> CreateContextFactory(GLFWwindow * window, dp::ApiVersion api, m2::PointU size);
void OnCreateDrapeEngine(GLFWwindow * window, dp::ApiVersion api, ref_ptr<dp::GraphicsContextFactory> contextFactory);
void PrepareDestroyContextFactory(ref_ptr<dp::GraphicsContextFactory> contextFactory);
void UpdateContentScale(GLFWwindow * window, float scale);
void UpdateSize(ref_ptr<dp::GraphicsContextFactory> contextFactory, int w, int h);

extern "C"
{
// ---------------------------------------------------------------------------
// Platform / logging.
// ---------------------------------------------------------------------------

uint32_t om_plat_cpu_cores(void)
{
  return static_cast<uint32_t>(GetPlatform().CpuCores());
}

void om_plat_version(char * buf, size_t cap)
{
  auto const version = GetPlatform().Version();
  std::memcpy(buf, version.c_str(), std::min(cap - 1, version.size()));
  buf[std::min(cap - 1, version.size())] = '\0';
}

void om_plat_setup_measurement(void)
{
  GetPlatform().SetupMeasurementSystem();
}

void om_plat_set_gui_thread(void * taskLoop)
{
  if (taskLoop == nullptr)
    return;
  std::unique_ptr<base::TaskLoop> loop(static_cast<SandboxTaskLoop *>(taskLoop));
  GetPlatform().SetGuiThread(std::move(loop));
}

void om_settings_dev_mode_set(int32_t enabled)
{
  settings::Set(settings::kDeveloperMode, enabled != 0);
}

int32_t om_settings_dev_mode_get(void)
{
  bool value = false;
  if (!settings::Get(settings::kDeveloperMode, value))
    return 0;
  return value ? 1 : 0;
}

void om_log_message(int32_t level, char const * msg, size_t len)
{
  auto const parsedLevel = static_cast<base::LogLevel>(level);
  LOG(parsedLevel, (std::string(msg, len)));
}

// ---------------------------------------------------------------------------
// GUI task loop.
// ---------------------------------------------------------------------------

OmTaskLoop * om_task_loop_new()
{
  return reinterpret_cast<OmTaskLoop *>(new SandboxTaskLoop());
}

void om_task_loop_execute(OmTaskLoop * tl)
{
  reinterpret_cast<SandboxTaskLoop *>(tl)->ExecuteTasks();
}

// ---------------------------------------------------------------------------
// Graphics context factory.
// ---------------------------------------------------------------------------

void * om_ctx_create(void * glfwWindow, int32_t apiVersion, uint32_t w, uint32_t h)
{
  auto factory = CreateContextFactory(static_cast<GLFWwindow *>(glfwWindow), static_cast<dp::ApiVersion>(apiVersion),
                                      m2::PointU(w, h));
  return factory.release();
}

void om_ctx_delete(void * ctxFactory)
{
  delete static_cast<dp::GraphicsContextFactory *>(ctxFactory);
}

void om_ctx_on_create_engine(void * glfwWindow, int32_t apiVersion, void * ctxFactory)
{
  if (ctxFactory == nullptr)
    return;
  auto contextFactory = ref_ptr<dp::GraphicsContextFactory>(static_cast<dp::GraphicsContextFactory *>(ctxFactory));
  OnCreateDrapeEngine(static_cast<GLFWwindow *>(glfwWindow), static_cast<dp::ApiVersion>(apiVersion), contextFactory);
}

void om_ctx_prepare_destroy(void * ctxFactory)
{
  if (ctxFactory == nullptr)
    return;
  auto contextFactory = ref_ptr<dp::GraphicsContextFactory>(static_cast<dp::GraphicsContextFactory *>(ctxFactory));
  PrepareDestroyContextFactory(contextFactory);
}

void om_ctx_update_content_scale(void * glfwWindow, float scale)
{
  UpdateContentScale(static_cast<GLFWwindow *>(glfwWindow), scale);
}

void om_ctx_update_size(void * ctxFactory, int w, int h)
{
  if (ctxFactory == nullptr)
    return;
  auto contextFactory = ref_ptr<dp::GraphicsContextFactory>(static_cast<dp::GraphicsContextFactory *>(ctxFactory));
  UpdateSize(contextFactory, w, h);
}

// ---------------------------------------------------------------------------
// Framework.
// ---------------------------------------------------------------------------

OmFramework * om_fw_new(int32_t enableDiffs)
{
  return reinterpret_cast<OmFramework *>(new SandboxFramework(enableDiffs != 0));
}

void om_fw_delete(OmFramework * f)
{
  auto * sandbox = reinterpret_cast<SandboxFramework *>(f);
  if (sandbox->subscribeId != 0)
    sandbox->framework.GetStorage().Unsubscribe(sandbox->subscribeId);
  sandbox->framework.DestroyDrapeEngine();
  delete sandbox->skin;
  delete sandbox;
}

void om_fw_set_callbacks(OmFramework * f, void * user, OmCountryChangedFn countryChanged,
                         OmDownloadProgressFn downloadProgress, OmRenderInjectionFn renderInjection)
{
  auto * sandbox = reinterpret_cast<SandboxFramework *>(f);
  sandbox->user = user;
  sandbox->onCountryChanged = countryChanged;
  sandbox->onDownloadProgress = downloadProgress;
  sandbox->onRenderInjection = renderInjection;

  sandbox->framework.SetCurrentCountryChangedListener([sandbox](storage::CountryId const & countryId)
  {
    if (sandbox->onCountryChanged != nullptr)
      sandbox->onCountryChanged(sandbox->user, countryId.c_str());
  });

  sandbox->subscribeId = sandbox->framework.GetStorage().Subscribe(
      [sandbox](storage::CountryId const & countryId)
  {
    // Storage notifies for parents too; only leaves are of interest.
    if (sandbox->framework.GetStorage().IsLeaf(countryId) && sandbox->onCountryChanged != nullptr)
      sandbox->onCountryChanged(sandbox->user, countryId.c_str());
  }, [sandbox](storage::CountryId const & countryId, downloader::Progress const & progress)
  {
    if (sandbox->onDownloadProgress != nullptr)
      sandbox->onDownloadProgress(sandbox->user, countryId.c_str(), progress.m_bytesDownloaded, progress.m_bytesTotal);
  });
}

int32_t om_fw_create_engine(OmFramework * f, void * contextFactory, int32_t apiVersion, double visualScale,
                            int surfaceWidth, int surfaceHeight)
{
  auto * sandbox = reinterpret_cast<SandboxFramework *>(f);
  if (sandbox->skin == nullptr)
    sandbox->skin = new gui::Skin(gui::ResolveGuiSkinFile("default"), static_cast<float>(visualScale));
  sandbox->skin->Resize(surfaceWidth, surfaceHeight);

  Framework::DrapeCreationParams params;
  params.m_apiVersion = static_cast<dp::ApiVersion>(apiVersion);
  params.m_visualScale = visualScale;
  params.m_surfaceWidth = surfaceWidth;
  params.m_surfaceHeight = surfaceHeight;
  params.m_renderInjectionHandler = [sandbox](ref_ptr<dp::GraphicsContext> context,
                                              ref_ptr<dp::TextureManager> textureManager,
                                              ref_ptr<gpu::ProgramManager> programManager, bool shutdown)
  {
    if (sandbox->onRenderInjection != nullptr)
      sandbox->onRenderInjection(sandbox->user, context.get(), textureManager.get(), programManager.get(),
                                 shutdown ? 1 : 0);
  };
  sandbox->skin->ForEach([&params](gui::EWidget widget, gui::Position const & pos)
  { params.m_widgetsInitInfo[widget] = pos; });
  params.m_widgetsInitInfo[gui::WIDGET_SCALE_FPS_LABEL] = gui::Position(dp::LeftTop);

  sandbox->framework.CreateDrapeEngine(
      ref_ptr<dp::GraphicsContextFactory>(static_cast<dp::GraphicsContextFactory *>(contextFactory)),
      std::move(params));
  return 0;
}

void om_fw_destroy_engine(OmFramework * f)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.DestroyDrapeEngine();
}

void om_fw_set_render_enabled(OmFramework * f)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.SetRenderingEnabled(nullptr);
}

void om_fw_set_render_disabled(OmFramework * f, int32_t destroySurface)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.SetRenderingDisabled(destroySurface != 0);
}

int32_t om_fw_api_version(OmFramework * f)
{
  auto * sandbox = reinterpret_cast<SandboxFramework *>(f);
  if (!sandbox->framework.IsDrapeEngineCreated())
    return OM_API_INVALID;
  return static_cast<int32_t>(sandbox->framework.GetDrapeEngine()->GetApiVersion());
}

void om_fw_on_size(OmFramework * f, int w, int h)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.OnSize(w, h);
}

void om_fw_update_visual_scale(OmFramework * f, double vs)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.UpdateVisualScale(vs);
}

void om_fw_update_widgets(OmFramework * f, int w, int h)
{
  auto * sandbox = reinterpret_cast<SandboxFramework *>(f);
  if (sandbox->skin == nullptr)
    return;
  sandbox->skin->Resize(w, h);
  gui::TWidgetsLayoutInfo layout;
  sandbox->skin->ForEach([&layout](gui::EWidget widget, gui::Position const & pos)
  { layout[widget] = pos.m_pixelPivot; });
  sandbox->framework.SetWidgetLayout(std::move(layout));
}

void om_fw_frame_active(OmFramework * f)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.MakeFrameActive();
}

void om_fw_enter_background(OmFramework * f)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.EnterBackground();
}

void om_fw_on_location(OmFramework * f, OmGpsInfo const * info)
{
  location::GpsInfo gps;
  gps.m_source = static_cast<location::TLocationSource>(info->source);
  gps.m_timestamp = info->timestamp;
  gps.m_latitude = info->latitude;
  gps.m_longitude = info->longitude;
  gps.m_horizontalAccuracy = info->horizontalAccuracy;
  gps.m_altitude = info->altitude;
  gps.m_verticalAccuracy = info->verticalAccuracy;
  gps.m_bearing = info->bearing;
  gps.m_speed = info->speed;
  reinterpret_cast<SandboxFramework *>(f)->framework.OnLocationUpdate(gps);
}

void om_fw_on_compass(OmFramework * f, OmCompassInfo const * info)
{
  location::CompassInfo compass;
  compass.m_bearing = info->bearing;
  reinterpret_cast<SandboxFramework *>(f)->framework.OnCompassUpdate(compass);
}

void om_fw_next_position_mode(OmFramework * f)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.SwitchMyPositionNextMode();
}

int32_t om_fw_position_mode(OmFramework * f)
{
  return static_cast<int32_t>(reinterpret_cast<SandboxFramework *>(f)->framework.GetMyPositionMode());
}

void om_fw_touch(OmFramework * f, OmTouchEvent const * ev)
{
  df::TouchEvent event;
  event.SetTouchType(static_cast<df::TouchEvent::ETouchType>(ev->type));

  df::Touch first;
  first.m_location = m2::PointF(ev->first.location.x, ev->first.location.y);
  first.m_id = ev->first.id;
  first.m_force = ev->first.force;
  event.SetFirstTouch(first);

  if (ev->hasSecond)
  {
    df::Touch second;
    second.m_location = m2::PointF(ev->second.location.x, ev->second.location.y);
    second.m_id = ev->second.id;
    second.m_force = ev->second.force;
    event.SetSecondTouch(second);
  }

  reinterpret_cast<SandboxFramework *>(f)->framework.TouchEvent(event);
}

void om_fw_scale(OmFramework * f, double factor, double px, double py, int32_t animated)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.Scale(factor, m2::PointD(px, py), animated != 0);
}

void om_fw_scale_zoom(OmFramework * f, int32_t magnify, int32_t animated)
{
  auto & framework = reinterpret_cast<SandboxFramework *>(f)->framework;
  framework.Scale(magnify ? Framework::SCALE_MAG : Framework::SCALE_MIN, animated != 0);
}

void om_fw_debug_rects(OmFramework * f, int32_t enabled)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.EnableDebugRectRendering(enabled != 0);
}

void om_fw_set_posteffect_aa(OmFramework * f, int32_t enabled)
{
  auto & framework = reinterpret_cast<SandboxFramework *>(f)->framework;
  if (framework.IsDrapeEngineCreated())
    framework.GetDrapeEngine()->SetPosteffectEnabled(df::PostprocessRenderer::Effect::Antialiasing, enabled != 0);
}

void om_fw_set_tile_background(OmFramework * f, int32_t mode, float opacity)
{
  auto & framework = reinterpret_cast<SandboxFramework *>(f)->framework;
  if (framework.IsDrapeEngineCreated())
    framework.GetDrapeEngine()->SetTileBackgroundMode(static_cast<dp::BackgroundMode>(mode), opacity);
}

OmPointD om_fw_pto_g(OmFramework * f, double x, double y)
{
  auto const result = reinterpret_cast<SandboxFramework *>(f)->framework.PtoG(m2::PointD(x, y));
  return OmPointD{result.x, result.y};
}

OmPointD om_fw_pixel_center(OmFramework * f)
{
  auto const result = reinterpret_cast<SandboxFramework *>(f)->framework.GetVisiblePixelCenter();
  return OmPointD{result.x, result.y};
}

int32_t om_fw_country_id_valid(char const * countryId)
{
  return storage::IsCountryIdValid(countryId) ? 1 : 0;
}

int32_t om_fw_country_status(OmFramework * f, char const * countryId)
{
  return static_cast<int32_t>(
      reinterpret_cast<SandboxFramework *>(f)->framework.GetStorage().CountryStatusEx(countryId));
}

int64_t om_fw_country_size(OmFramework * f, char const * countryId)
{
  return reinterpret_cast<SandboxFramework *>(f)->framework.GetStorage().CountrySizeInBytes(countryId).second;
}

void om_fw_download_country(OmFramework * f, char const * countryId)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.GetStorage().DownloadNode(countryId);
}

void om_fw_retry_download_country(OmFramework * f, char const * countryId)
{
  reinterpret_cast<SandboxFramework *>(f)->framework.GetStorage().RetryDownloadNode(countryId);
}

// ---------------------------------------------------------------------------
// ImGui -> drape backend.
// ---------------------------------------------------------------------------

void * om_imgui_new(void)
{
  return new ImguiRenderer();
}

void om_imgui_delete(void * renderer)
{
  delete static_cast<ImguiRenderer *>(renderer);
}

void om_imgui_set_texture(void * renderer, uint32_t width, uint32_t height, uint8_t const * rgba, size_t len)
{
  static_cast<ImguiRenderer *>(renderer)->SetTexture(width, height, rgba, len);
}

void om_imgui_update(void * renderer, OmImGuiDrawList const * lists, uint32_t listCount, float displayPosX,
                     float displayPosY, float displaySizeX, float displaySizeY, float framebufferScaleX,
                     float framebufferScaleY)
{
  static_cast<ImguiRenderer *>(renderer)->Update(lists, listCount, displayPosX, displayPosY, displaySizeX, displaySizeY,
                                                 framebufferScaleX, framebufferScaleY);
}

void om_imgui_render(void * renderer, void * context, void * textureManager, void * programManager)
{
  static_cast<ImguiRenderer *>(renderer)->Render(
      ref_ptr<dp::GraphicsContext>(static_cast<dp::GraphicsContext *>(context)),
      ref_ptr<dp::TextureManager>(static_cast<dp::TextureManager *>(textureManager)),
      ref_ptr<gpu::ProgramManager>(static_cast<gpu::ProgramManager *>(programManager)));
}

void om_imgui_reset(void * renderer)
{
  static_cast<ImguiRenderer *>(renderer)->Reset();
}

}  // extern "C"
