#pragma once

#include <QWidget>

class QToolButton;
class QLineEdit;

namespace qt::common
{
class ZoomWidget : public QWidget
{
  Q_OBJECT

public:
  explicit ZoomWidget(QWidget * parent = nullptr);

  void SetZoomLevel(int level, int minLevel, int maxLevel);

Q_SIGNALS:
  void ZoomIn();
  void ZoomOut();
  void ZoomToLevel(int level);

protected:
  void paintEvent(QPaintEvent *) override;

private:
  void OnTextConfirmed();
  void OnEditingFinished();

  QToolButton * m_zoomInBtn;
  QToolButton * m_zoomOutBtn;
  QLineEdit * m_zoomText;

  int m_level = 0;
  int m_minLevel = 0;
  int m_maxLevel = 1;
};
}  // namespace qt::common
