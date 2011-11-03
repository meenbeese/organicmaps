#include "../base/SRC_FIRST.hpp"

#include "render_state_updater.hpp"
#include "render_state.hpp"
#include "framebuffer.hpp"
#include "base_texture.hpp"

#include "internal/opengl.hpp"

#include "../base/logging.hpp"
#include "../std/bind.hpp"

namespace yg
{
  namespace gl
  {
    RenderStateUpdater::Params::Params()
      : m_doPeriodicalUpdate(false), m_updateInterval(0.0)
    {}

    RenderStateUpdater::RenderStateUpdater(Params const & params)
      : base_t(params),
      m_renderState(params.m_renderState),
      m_doPeriodicalUpdate(params.m_doPeriodicalUpdate),
      m_updateInterval(params.m_updateInterval)
    {
    }

    shared_ptr<RenderState> const & RenderStateUpdater::renderState() const
    {
      return m_renderState;
    }

    void RenderStateUpdater::drawGeometry(shared_ptr<BaseTexture> const & texture,
                                          shared_ptr<VertexBuffer> const & vertices,
                                          shared_ptr<IndexBuffer> const & indices,
                                          size_t indicesCount)
    {
      base_t::drawGeometry(texture, vertices, indices, indicesCount);
      m_indicesCount += indicesCount;
      if (m_doPeriodicalUpdate
       && m_renderState
       && (m_indicesCount > 20000)
       && (m_updateTimer.ElapsedSeconds() > m_updateInterval))
      {
        updateActualTarget();
        m_indicesCount %= 20000;
        m_updateTimer.Reset();
      }
    }

    void RenderStateUpdater::UpdateActualTarget::perform()
    {
      if (m_doSynchronize)
        m_renderState->m_mutex->Lock();
      m_renderState->m_actualScreen = m_currentScreen;
      if (m_doSynchronize)
        m_renderState->m_mutex->Unlock();
    }

    void RenderStateUpdater::UpdateBackBuffer::perform()
    {
      if (isDebugging())
        LOG(LINFO, ("performing UpdateBackBuffer command"));

      OGLCHECK(glFinish());

      OGLCHECK(glDisable(GL_SCISSOR_TEST));

      OGLCHECK(glClearColor(192 / 255.0, 192 / 255.0, 192 / 255.0, 1.0));
      OGLCHECK(glClear(GL_COLOR_BUFFER_BIT));

      shared_ptr<IMMDrawTexturedRect> immDrawTexturedRect(
            new IMMDrawTexturedRect(m2::RectF(0, 0, m_actualTarget->width(), m_actualTarget->height()),
                                    m2::RectF(0, 0, 1, 1),
                                    m_actualTarget,
                                    m_resourceManager));

      immDrawTexturedRect->perform();

      if (m_isClipRectEnabled)
        OGLCHECK(glEnable(GL_SCISSOR_TEST));

      OGLCHECK(glFinish());

      m_renderState->invalidate();
    }

    void RenderStateUpdater::updateActualTarget()
    {
      /// Carefully synchronizing the access to the m_renderState to minimize wait time.
      processCommand(shared_ptr<Command>(new FinishCommand()));

      m_renderState->m_mutex->Lock();

      swap(m_renderState->m_actualTarget, m_renderState->m_backBufferLayers.front());

      shared_ptr<UpdateActualTarget> command(new UpdateActualTarget());
      command->m_renderState = m_renderState;
      command->m_currentScreen = m_renderState->m_currentScreen;
      command->m_doSynchronize = renderQueue();

      processCommand(command);

      shared_ptr<UpdateBackBuffer> command1(new UpdateBackBuffer());
      command1->m_actualTarget = m_renderState->m_actualTarget;
      command1->m_renderState = m_renderState;
      command1->m_resourceManager = resourceManager();
      command1->m_isClipRectEnabled = clipRectEnabled();

      /// blitting will be performed through
      /// non-multisampled framebuffer for the sake of speed
      setRenderTarget(m_renderState->m_backBufferLayers.front());

//    m_renderState->m_actualScreen = m_renderState->m_currentScreen;
      m_renderState->m_mutex->Unlock();

      processCommand(command1);
    }

    void RenderStateUpdater::beginFrame()
    {
      base_t::beginFrame();
      m_indicesCount = 0;
      m_updateTimer.Reset();
    }

    void RenderStateUpdater::setClipRect(m2::RectI const & rect)
    {
      if ((m_renderState) && (m_indicesCount))
      {
        updateActualTarget();
        m_indicesCount = 0;
        m_updateTimer.Reset();
      }

      base_t::setClipRect(rect);
    }

    void RenderStateUpdater::endFrame()
    {
      if (m_renderState)
        updateActualTarget();
      m_indicesCount = 0;
      m_updateTimer.Reset();
      base_t::endFrame();
    }
  }
}
