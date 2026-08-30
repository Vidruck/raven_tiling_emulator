qt_policy(SET QTP0001 NEW)

set_source_files_properties(RavenTheme.qml PROPERTIES
    QT_QML_SINGLETON_TYPE TRUE
)

qt_add_qml_module(ravenplugin
    URI "org.kde.plasma.ravenlauncher.plugin"
    VERSION 1.0
    OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/org/kde/plasma/ravenlauncher/plugin"
    SOURCES
        ravencontroller.cpp
        ravencontroller.h
        systemcontroller.cpp
        systemcontroller.h
        apprunner.cpp
        apprunner.h
        mediacontroller.cpp
        mediacontroller.h
        systemstats.cpp
        systemstats.h
        colorextractor.cpp
        colorextractor.h
        weathercontroller.cpp
        weathercontroller.h

    QML_FILES
        RavenTheme.qml
)

target_link_libraries(ravenplugin
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::DBus
        Qt6::Network
        Qt6::Concurrent
)

# Set rpath so ravenpluginplugin finds libravenplugin in the same directory
set_target_properties(ravenplugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/org/kde/plasma/ravenlauncher/plugin"
    BUILD_WITH_INSTALL_RPATH TRUE
    INSTALL_RPATH "$ORIGIN"
)
set_target_properties(ravenpluginplugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/org/kde/plasma/ravenlauncher/plugin"
    PREFIX "lib"
    BUILD_WITH_INSTALL_RPATH TRUE
    INSTALL_RPATH "$ORIGIN"
)

install(TARGETS ravenplugin ravenpluginplugin DESTINATION lib/qt6/qml/org/kde/plasma/ravenlauncher/plugin)
install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/RavenTheme.qml
              ${CMAKE_CURRENT_BINARY_DIR}/org/kde/plasma/ravenlauncher/plugin/qmldir
              ${CMAKE_CURRENT_BINARY_DIR}/org/kde/plasma/ravenlauncher/plugin/ravenplugin.qmltypes
        DESTINATION lib/qt6/qml/org/kde/plasma/ravenlauncher/plugin)

install(TARGETS ravenplugin ravenpluginplugin DESTINATION lib64/qt6/qml/org/kde/plasma/ravenlauncher/plugin)
install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/RavenTheme.qml
              ${CMAKE_CURRENT_BINARY_DIR}/org/kde/plasma/ravenlauncher/plugin/qmldir
              ${CMAKE_CURRENT_BINARY_DIR}/org/kde/plasma/ravenlauncher/plugin/ravenplugin.qmltypes
        DESTINATION lib64/qt6/qml/org/kde/plasma/ravenlauncher/plugin)

