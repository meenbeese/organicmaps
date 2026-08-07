#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

// ---------------------------------------------------------------------------
// Enums (mirror the C++ values they stand for).
// ---------------------------------------------------------------------------

// base::LogLevel
enum
{
  OM_LOG_DEBUG = 0,
  OM_LOG_INFO,
  OM_LOG_WARNING,
  OM_LOG_ERROR,
  OM_LOG_CRITICAL,
};

// dp::ApiVersion
enum
{
  OM_API_INVALID = -1,
  OM_API_OPENGLES3 = 0,
  OM_API_METAL = 1,
  OM_API_VULKAN = 2,
};

// df::TouchEvent::ETouchType
enum
{
  OM_TOUCH_NONE = 0,
  OM_TOUCH_DOWN,
  OM_TOUCH_MOVE,
  OM_TOUCH_UP,
  OM_TOUCH_CANCEL,
};

// location::EMyPositionMode
enum
{
  OM_POSITION_PENDING = 0,
  OM_POSITION_NOT_FOLLOW_NO_POS,
  OM_POSITION_NOT_FOLLOW,
  OM_POSITION_FOLLOW,
  OM_POSITION_FOLLOW_AND_ROTATE,
};

// storage::Status (inner status returned by CountryStatusEx)
enum
{
  OM_STATUS_UNDEFINED = 0,
  OM_STATUS_ON_DISK,
  OM_STATUS_NOT_DOWNLOADED,
  OM_STATUS_DOWNLOAD_FAILED,
  OM_STATUS_DOWNLOADING,
  OM_STATUS_APPLYING,
  OM_STATUS_IN_QUEUE,
  OM_STATUS_UNKNOWN_ERROR,
  OM_STATUS_ON_DISK_OUT_OF_DATE,
  OM_STATUS_OUT_OF_MEM_FAILED,
};

// dp::BackgroundMode
enum
{
  OM_BACKGROUND_DEFAULT = 0,
  OM_BACKGROUND_SATELLITE,
};

// ---------------------------------------------------------------------------
// Plain data structs (POD, layout matches the C++ counterparts).
// ---------------------------------------------------------------------------

typedef struct OmPointD
{
  double x;
  double y;
} OmPointD;

typedef struct OmPointF
{
  float x;
  float y;
} OmPointF;

typedef struct OmTouch
{
  OmPointF location;
  int64_t id;
  float force;
} OmTouch;

typedef struct OmTouchEvent
{
  int32_t type;  // OmTouchType
  OmTouch first;
  OmTouch second;  // valid when hasSecond != 0
  int32_t hasSecond;
} OmTouchEvent;

typedef struct OmGpsInfo
{
  int32_t source;  // location::TLocationSource
  double timestamp;
  double latitude;
  double longitude;
  double horizontalAccuracy;
  double altitude;
  double verticalAccuracy;
  double bearing;  // degrees from true North, -1.0 when unknown
  double speed;
} OmGpsInfo;

typedef struct OmCompassInfo
{
  double bearing;  // radians from true North
} OmCompassInfo;

// ---------------------------------------------------------------------------
// Platform / logging.
// ---------------------------------------------------------------------------

uint32_t om_plat_cpu_cores(void);
void om_plat_version(char * buf, size_t cap);
void om_plat_setup_measurement(void);
void om_plat_set_gui_thread(void * taskLoop);
void om_settings_dev_mode_set(int32_t enabled);
int32_t om_settings_dev_mode_get(void);
void om_log_message(int32_t level, char const * msg, size_t len);

// ---------------------------------------------------------------------------
// GUI task loop. `om_plat_set_gui_thread` takes the `om_task_loop_new` result;
// Rust drains it every frame via `om_task_loop_execute` on the main thread.
// ---------------------------------------------------------------------------

typedef struct OmTaskLoop OmTaskLoop;

OmTaskLoop * om_task_loop_new(void);
void om_task_loop_execute(OmTaskLoop * tl);

// ---------------------------------------------------------------------------
// Graphics context factory (implemented by the per-platform C++ shims).
// ---------------------------------------------------------------------------

void * om_ctx_create(void * glfwWindow, int32_t apiVersion, uint32_t w, uint32_t h);
void om_ctx_delete(void * ctxFactory);
void om_ctx_on_create_engine(void * glfwWindow, int32_t apiVersion, void * ctxFactory);
void om_ctx_prepare_destroy(void * ctxFactory);
void om_ctx_update_content_scale(void * glfwWindow, float scale);
void om_ctx_update_size(void * ctxFactory, int w, int h);

// ---------------------------------------------------------------------------
// Framework.
// ---------------------------------------------------------------------------

typedef struct OmFramework OmFramework;

typedef void (*OmCountryChangedFn)(void * user, char const * countryId);
typedef void (*OmDownloadProgressFn)(void * user, char const * countryId, int64_t downloaded, int64_t total);
typedef void (*OmRenderInjectionFn)(void * user, void * context, void * textureManager, void * programManager,
                                    int32_t shutdown);

OmFramework * om_fw_new(int32_t enableDiffs);
void om_fw_delete(OmFramework * f);
void om_fw_set_callbacks(OmFramework * f, void * user, OmCountryChangedFn countryChanged,
                         OmDownloadProgressFn downloadProgress, OmRenderInjectionFn renderInjection);

int32_t om_fw_create_engine(OmFramework * f, void * contextFactory, int32_t apiVersion, double visualScale,
                            int surfaceWidth, int surfaceHeight);
void om_fw_destroy_engine(OmFramework * f);
void om_fw_set_render_enabled(OmFramework * f);
void om_fw_set_render_disabled(OmFramework * f, int32_t destroySurface);
int32_t om_fw_api_version(OmFramework * f);
void om_fw_on_size(OmFramework * f, int w, int h);
void om_fw_update_visual_scale(OmFramework * f, double vs);
void om_fw_update_widgets(OmFramework * f, int w, int h);
void om_fw_frame_active(OmFramework * f);
void om_fw_enter_background(OmFramework * f);

void om_fw_on_location(OmFramework * f, OmGpsInfo const * info);
void om_fw_on_compass(OmFramework * f, OmCompassInfo const * info);
void om_fw_next_position_mode(OmFramework * f);
int32_t om_fw_position_mode(OmFramework * f);

void om_fw_touch(OmFramework * f, OmTouchEvent const * ev);
void om_fw_scale(OmFramework * f, double factor, double px, double py, int32_t animated);
void om_fw_scale_zoom(OmFramework * f, int32_t magnify, int32_t animated);
void om_fw_debug_rects(OmFramework * f, int32_t enabled);
void om_fw_set_posteffect_aa(OmFramework * f, int32_t enabled);
void om_fw_set_tile_background(OmFramework * f, int32_t mode, float opacity);
OmPointD om_fw_pto_g(OmFramework * f, double x, double y);
OmPointD om_fw_pixel_center(OmFramework * f);

int32_t om_fw_country_id_valid(char const * countryId);
int32_t om_fw_country_status(OmFramework * f, char const * countryId);
int64_t om_fw_country_size(OmFramework * f, char const * countryId);
void om_fw_download_country(OmFramework * f, char const * countryId);
void om_fw_retry_download_country(OmFramework * f, char const * countryId);

// ---------------------------------------------------------------------------
// ImGui -> drape backend. The UI runs in Rust (imgui-rs); its draw data is
// pushed into the C++ renderer (in c_api.cpp) for drawing on the render thread.
// The struct layouts match the imgui-rs types they are fed from.
// ---------------------------------------------------------------------------

typedef struct OmImGuiVertex
{
  float x, y;      // position
  float u, v;      // texture coords
  uint32_t color;  // raw ImGui color (byte order reversed vs. RGBA)
} OmImGuiVertex;

typedef struct OmImGuiCmd
{
  float clipX, clipY, clipZ, clipW;  // left, up, right, down in display coords
  uint32_t elemCount;
  uint32_t idxOffset;  // offset of the first index within the draw list
} OmImGuiCmd;

typedef struct OmImGuiDrawList
{
  OmImGuiVertex const * vertices;
  uint32_t vertexCount;
  uint16_t const * indices;
  uint32_t indexCount;
  OmImGuiCmd const * cmds;
  uint32_t cmdCount;
} OmImGuiDrawList;

void * om_imgui_new(void);
void om_imgui_delete(void * renderer);
void om_imgui_set_texture(void * renderer, uint32_t width, uint32_t height, uint8_t const * rgba, size_t len);
void om_imgui_update(void * renderer, OmImGuiDrawList const * lists, uint32_t listCount, float displayPosX,
                     float displayPosY, float displaySizeX, float displaySizeY, float framebufferScaleX,
                     float framebufferScaleY);
void om_imgui_render(void * renderer, void * context, void * textureManager, void * programManager);
void om_imgui_reset(void * renderer);

#ifdef __cplusplus
}  // extern "C"
#endif
