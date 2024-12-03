cmake_minimum_required(VERSION 3.22.1)
project(desktop)

find_package(Slint REQUIRED)

set(SRC
  about.cpp
  about.hpp
  bookmark_dialog.cpp
  bookmark_dialog.hpp
  build_style/build_common.cpp
  build_style/build_common.h
  build_style/build_drules.cpp
  build_style/build_drules.h
  build_style/build_phone_pack.cpp
  build_style/build_phone_pack.h
  build_style/build_skins.cpp
  build_style/build_skins.h
  build_style/build_statistics.cpp
  build_style/build_statistics.h
  build_style/build_style.cpp
  build_style/build_style.h
  build_style/run_tests.cpp
  build_style/run_tests.h
  create_feature_dialog.cpp
  create_feature_dialog.hpp
  draw_widget.cpp
  draw_widget.hpp
  editor_dialog.cpp
  editor_dialog.hpp
  info_dialog.cpp
  info_dialog.hpp
  main.cpp
  mainwindow.cpp
  mainwindow.hpp
  mwms_borders_selection.cpp
  mwms_borders_selection.hpp
  osm_auth_dialog.cpp
  osm_auth_dialog.hpp
  place_page_dialog_common.cpp
  place_page_dialog_common.hpp
  place_page_dialog_developer.cpp
  place_page_dialog_developer.hpp
  place_page_dialog_user.cpp
  place_page_dialog_user.hpp
  preferences_dialog.cpp
  preferences_dialog.hpp
  popup_menu_holder.cpp
  popup_menu_holder.hpp
  routing_settings_dialog.cpp
  routing_settings_dialog.hpp
  routing_turns_visualizer.cpp
  routing_turns_visualizer.hpp
  ruler.cpp
  ruler.hpp
  screenshoter.cpp
  screenshoter.hpp
  search_panel.cpp
  search_panel.hpp
  selection.hpp
  update_dialog.cpp
  update_dialog.hpp
)

set(SLINT_UI
  res/main.slint
  # Add more .slint files as needed
)

slint_generate_cpp(UI_SOURCES ${SLINT_UI})

add_executable(${PROJECT_NAME} ${SRC} ${UI_SOURCES})

set_target_properties(${PROJECT_NAME} PROPERTIES
    CXX_STANDARD 20
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
)

target_link_libraries(${PROJECT_NAME} PRIVATE Slint::Slint)

target_link_libraries(${PROJECT_NAME} PRIVATE
    generator
    qt_common
    map
    gflags::gflags
)

if (APPLE)
    set(BUNDLE_NAME "OMaps")
    set(BUNDLE_DISPLAY_NAME "Organic Maps Desktop")
    set_target_properties(${PROJECT_NAME} PROPERTIES
        MACOSX_BUNDLE TRUE
        OUTPUT_NAME ${BUNDLE_NAME}
        MACOSX_BUNDLE_GUI_IDENTIFIER "com.organicmaps.desktop"
    )
endif()

set(DATA_DIR ${CMAKE_SOURCE_DIR}/data)
set(RESOURCES_DIR ${CMAKE_BINARY_DIR}/resources)
file(MAKE_DIRECTORY ${RESOURCES_DIR})

function(copy_resources)
    foreach(file ${ARGN})
        configure_file(${DATA_DIR}/${file} ${RESOURCES_DIR}/${file} COPYONLY)
    endforeach()
endfunction()

copy_resources(
    countries-strings
    resources-default
    resources-mdpi_light
    resources-hdpi_light
    resources-xhdpi_light
    resources-xxhdpi_light
    resources-xxxhdpi_light
    resources-6plus_light
    welcome.html

    categories.txt
    categories_cuisines.txt
    classificator.txt
    colors.txt
    copyright.html
    countries.txt
    drules_proto_default_light.bin
    drules_proto_default_dark.bin
    drules_proto_vehicle_light.bin
    drules_proto_vehicle_dark.bin
    editor.config
    fonts_blacklist.txt
    fonts_whitelist.txt
    packed_polygons.bin
    patterns.txt
    transit_colors.txt
    types.txt
    unicode_blocks.txt
    World.mwm
    WorldCoasts.mwm

    00_NotoNaskhArabic-Regular.ttf
    00_NotoSansBengali-Regular.ttf
    00_NotoSansHebrew-Regular.ttf
    00_NotoSansMalayalam-Regular.ttf
    00_NotoSansThai-Regular.ttf
    00_NotoSerifDevanagari-Regular.ttf
    01_dejavusans.ttf
    02_droidsans-fallback.ttf
    03_jomolhari-id-a3d.ttf
    04_padauk.ttf
    05_khmeros.ttf
    06_code2000.ttf
    07_roboto_medium.ttf
)

install(TARGETS ${PROJECT_NAME} DESTINATION bin)
install(DIRECTORY ${RESOURCES_DIR}/ DESTINATION share/organicmaps/resources)
install(DIRECTORY ${DATA_DIR} DESTINATION share/organicmaps/data)

add_subdirectory(qt_common)
