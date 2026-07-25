#include "qt/qt_common/zoom_widget.hpp"

#include <QHBoxLayout>
#include <QLineEdit>
#include <QPainter>
#include <QPainterPath>
#include <QToolButton>

namespace qt::common
{
namespace
{
constexpr int kWidgetHeight = 32;
constexpr int kWidgetWidth = 108;

constexpr char const * kButtonTextStyle = R"(
  QToolButton {
    background: transparent;
    color: white;
    border: none;
    font-size: 16px;
    font-weight: bold;
  }
)";

constexpr char const * kTextStyle = R"(
  QLineEdit {
    background: transparent;
    color: white;
    border: none;
    font-size: 13px;
    font-weight: bold;
    qproperty-alignment: AlignCenter;
    padding: 0;
    selection-background-color: rgba(255, 255, 255, 80);
  }
)";
}  // namespace

ZoomWidget::ZoomWidget(QWidget * parent) : QWidget(parent)
{
  setAttribute(Qt::WA_TranslucentBackground);
  setAttribute(Qt::WA_NoSystemBackground);

  auto * layout = new QHBoxLayout(this);
  layout->setContentsMargins(1, 1, 1, 1);
  layout->setSpacing(0);

  m_zoomOutBtn = new QToolButton();
  m_zoomOutBtn->setStyleSheet(kButtonTextStyle);
  m_zoomOutBtn->setText(QString::fromUtf8("\xe2\x88\x92"));
  connect(m_zoomOutBtn, &QToolButton::clicked, this, &ZoomWidget::ZoomOut);

  m_zoomText = new QLineEdit();
  m_zoomText->setStyleSheet(kTextStyle);
  m_zoomText->setReadOnly(false);
  m_zoomText->setFixedWidth(36);
  connect(m_zoomText, &QLineEdit::returnPressed, this, &ZoomWidget::OnTextConfirmed);
  connect(m_zoomText, &QLineEdit::editingFinished, this, &ZoomWidget::OnEditingFinished);

  m_zoomInBtn = new QToolButton();
  m_zoomInBtn->setStyleSheet(kButtonTextStyle);
  m_zoomInBtn->setText(QString::fromUtf8("\xe2\x80\x89+\xe2\x80\x89"));
  connect(m_zoomInBtn, &QToolButton::clicked, this, &ZoomWidget::ZoomIn);

  layout->addWidget(m_zoomOutBtn);
  layout->addWidget(m_zoomText);
  layout->addWidget(m_zoomInBtn);

  setFixedHeight(kWidgetHeight);
  setFixedWidth(kWidgetWidth);
}

void ZoomWidget::paintEvent(QPaintEvent *)
{
  QPainter p(this);
  p.setRenderHint(QPainter::Antialiasing);
  p.setBrush(QColor(0, 0, 0, 160));
  p.setPen(Qt::NoPen);
  QPainterPath path;
  path.addRoundedRect(rect(), height() / 2.0, height() / 2.0);
  p.drawPath(path);
}

void ZoomWidget::SetZoomLevel(int level, int minLevel, int maxLevel)
{
  m_level = level;
  m_minLevel = minLevel;
  m_maxLevel = maxLevel;

  m_zoomText->blockSignals(true);
  m_zoomText->setText(QString::number(level));
  m_zoomText->blockSignals(false);

  m_zoomOutBtn->setEnabled(level > minLevel);
  m_zoomInBtn->setEnabled(level < maxLevel);
}

void ZoomWidget::OnTextConfirmed()
{
  bool ok = false;
  int level = m_zoomText->text().toInt(&ok);
  if (ok && level >= m_minLevel && level <= m_maxLevel && level != m_level)
    Q_EMIT ZoomToLevel(level);
  else
    m_zoomText->setText(QString::number(m_level));

  clearFocus();
}

void ZoomWidget::OnEditingFinished()
{
  if (m_zoomText->isModified())
    OnTextConfirmed();
}
}  // namespace qt::common
