cmake_minimum_required(VERSION 3.16)

project(kwin-effect-raven VERSION 3.4.0 LANGUAGES CXX)

set(QT_MAJOR_VERSION 6)
set(REQUIRED_QT_VERSION "6.5.0")
set(KF_MIN_VERSION "6.0.0")

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(ECM 0.0.11 REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)

find_package(Qt6 ${REQUIRED_QT_VERSION} REQUIRED COMPONENTS
    Core
    Gui
    Widgets
    DBus
    Quick
    OpenGL
)

find_package(KF6 ${KF_MIN_VERSION} REQUIRED COMPONENTS
    Config
    CoreAddons
    WindowSystem
)

find_package(KWin REQUIRED)

# Requerido por kcoreaddons_add_plugin / KDECMakeSettings
if(NOT CMAKE_LIBRARY_OUTPUT_DIRECTORY)
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR})
endif()
 
# kcoreaddons_add_plugin crea el MODULE y establece
# KPLUGINFACTORY_PLUGIN_CLASS_INTERNAL_NAME que necesita KWIN_EFFECT_FACTORY_SUPPORTED
kcoreaddons_add_plugin(kwin4_effect_raven
    SOURCES
        src/raveneffect.cpp
        src/raveneffect_factory.cpp
        src/dbus/raven_effect_adaptor.cpp
    INSTALL_NAMESPACE "kwin/effects/plugins"
)

target_include_directories(kwin4_effect_raven PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${CMAKE_CURRENT_SOURCE_DIR}/src/dbus
)

target_link_libraries(kwin4_effect_raven
    PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Widgets
    Qt6::DBus
    Qt6::Quick
    KF6::ConfigCore
    KF6::CoreAddons
    KF6::WindowSystem
    KWin::kwin
)
