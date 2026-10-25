# FindVulkan fallback stub para KWin effect en sistemas sin vulkan-headers instalados
if(NOT TARGET Vulkan::Vulkan)
    add_library(Vulkan::Vulkan INTERFACE IMPORTED)
endif()
if(NOT TARGET Vulkan::Headers)
    add_library(Vulkan::Headers INTERFACE IMPORTED)
endif()

set(Vulkan_FOUND TRUE CACHE BOOL "Vulkan found" FORCE)
set(VULKAN_FOUND TRUE CACHE BOOL "Vulkan found" FORCE)
set(Vulkan_VERSION "1.3.0" CACHE STRING "Vulkan version" FORCE)
set(Vulkan_INCLUDE_DIRS "${CMAKE_CURRENT_SOURCE_DIR}/cmake/include" CACHE PATH "Vulkan include dirs" FORCE)
set(Vulkan_INCLUDE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/cmake/include" CACHE PATH "Vulkan include dir" FORCE)
set(Vulkan_LIBRARIES "vulkan" CACHE STRING "Vulkan libraries" FORCE)
set(Vulkan_LIBRARY "vulkan" CACHE FILEPATH "Vulkan library" FORCE)

set_target_properties(Vulkan::Vulkan PROPERTIES
    INTERFACE_INCLUDE_DIRECTORIES "${Vulkan_INCLUDE_DIRS}"
)
set_target_properties(Vulkan::Headers PROPERTIES
    INTERFACE_INCLUDE_DIRECTORIES "${Vulkan_INCLUDE_DIRS}"
)
