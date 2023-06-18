#pragma once

#include "geometry/point3d.hpp"

#include <optional>
#include <string>

namespace df
{
// Declaration for custom Arrow3D object.
struct Arrow3dCustomDecl
{
  // Path to arrow mesh .OBJ file.
  std::string m_arrowMeshPath;
  // Path to arrow mesh texture .PNG file.
  // If it's not set, default arrow color is used.
  std::optional<std::string> m_arrowMeshTexturePath;
  // Path to shadow mesh .OBJ file.
  // If it's not set, no shadow or outline will be rendered.
  std::optional<std::string> m_shadowMeshPath;
  
  // Leyout is axes (in the plane of map): x - right, y - up,
  // -z - perpendicular to the map's plane directed towards the observer.
  
  // Offset is in local (model's) coordinates.
  m3::PointF m_offset = m3::PointF(0.0f, 0.0f, 0.0f);
  // Rotation angles.
  m3::PointF m_eulerAngles = m3::PointF(0.0f, 0.0f, 0.0f);
  // Scale values.
  m3::PointF m_scale = m3::PointF(1.0f, 1.0f, 1.0f);

  // Flip U texture coordinate.
  bool m_flipTexCoordU = false;
  // Flip V texture coordinate (enabled in the Drape by default).
  bool m_flipTexCoordV = true;
  
  // Enable shadow rendering (only in perspective mode).
  // Shadow mesh must exist, otherwise standard one will be used.
  bool m_enableShadow = false;
  // Enabled outlint rendering (only in routing mode).
  // Shadow mesh must exist, otherwise standard one will be used.
  bool m_enableOutline = false;
};
}  // namespace df
