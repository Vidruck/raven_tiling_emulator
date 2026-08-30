cmake_minimum_required(VERSION 3.16)

project(ravenlauncher VERSION 1.0.0 LANGUAGES CXX)

set(QT_MAJOR_VERSION 6)
set(REQUIRED_QT_VERSION "6.5.0")

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(ECM 0.0.11 REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)

find_package(Qt6 ${REQUIRED_QT_VERSION} REQUIRED COMPONENTS Core Gui Qml DBus Quick Network Concurrent)
find_package(Plasma REQUIRED)

plasma_install_package(package org.kde.plasma.ravenlauncher)

add_subdirectory(plugin)
