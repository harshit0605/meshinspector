cmake_minimum_required(VERSION 3.16 FATAL_ERROR)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(POLICY CMP0167)
    cmake_policy(SET CMP0167 NEW)
endif()

project(meshinspector_meshlib_workbench CXX)
set(CMAKE_FIND_PACKAGE_PREFER_CONFIG TRUE)

if(EMSCRIPTEN)
    set(HTML_DIR "${CMAKE_BINARY_DIR}/html")
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${HTML_DIR})
endif()

if(DEFINED MESHLIB_SDK_ROOT AND EXISTS "${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib")
    list(APPEND CMAKE_MODULE_PATH "${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib/Modules")
    set(MESHLIB_INCLUDE_DIR "${MESHLIB_SDK_ROOT}/include/MeshLib;${MESHLIB_SDK_ROOT}/include/MeshLib/imgui")
    set(MESHLIB_THIRDPARTY_INCLUDE_DIR "${MESHLIB_SDK_ROOT}/include")
    set(MESHLIB_THIRDPARTY_LIB_DIR "${MESHLIB_SDK_ROOT}/lib")
    set(MESHLIB_DATA_DIR "${MESHLIB_SDK_ROOT}/share/MeshLib")
    set(MESHLIB_EMSCRIPTEN_CXX_FLAGS "--use-port=boost_headers --use-port=freetype --use-port=libpng --use-port=zlib")
    set(MESHLIB_EMSCRIPTEN_EXE_LINKER_FLAGS "-s EXPORTED_RUNTIME_METHODS=[ccall] -s ALLOW_MEMORY_GROWTH=1 -s LLD_REPORT_UNDEFINED=1 -s USE_WEBGL2=1 -s USE_GLFW=3 -s FULL_ES3=1 -s STACK_SIZE=1048576 -s ENVIRONMENT=web,node -s ASYNCIFY -Wno-limited-postlink-optimizations")

    include(CMakeFindDependencyMacro)
    include(EmscriptenHelpers)
    include("${MESHLIB_SDK_ROOT}/lib/cmake/glad/gladConfig.cmake")
    include("${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib/MRMeshConfig.cmake")
    include("${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib/MRIOExtrasConfig.cmake")

    if(NOT TARGET Freetype::Freetype)
        add_library(Freetype::Freetype INTERFACE IMPORTED)
    endif()
    if(NOT TARGET OpenVDB::openvdb)
        add_library(OpenVDB::openvdb STATIC IMPORTED)
        set_target_properties(OpenVDB::openvdb PROPERTIES IMPORTED_LOCATION "${MESHLIB_SDK_ROOT}/lib/libopenvdb.a")
    endif()
    if(NOT TARGET blosc)
        add_library(blosc STATIC IMPORTED)
        set_target_properties(blosc PROPERTIES IMPORTED_LOCATION "${MESHLIB_SDK_ROOT}/lib/libblosc.a")
    endif()
    foreach(_gdcm_target gdcmCommon gdcmIOD gdcmDICT gdcmDSED gdcmMEXD gdcmMSFF gdcmcharls gdcmexpat gdcmjpeg8 gdcmjpeg12 gdcmjpeg16 gdcmopenjp2 gdcmuuid jpeg socketxx)
        if(NOT TARGET ${_gdcm_target})
            add_library(${_gdcm_target} STATIC IMPORTED)
            set_target_properties(${_gdcm_target} PROPERTIES IMPORTED_LOCATION "${MESHLIB_SDK_ROOT}/lib/lib${_gdcm_target}.a")
        endif()
    endforeach()
    set_target_properties(gdcmDICT PROPERTIES INTERFACE_LINK_LIBRARIES "gdcmCommon")
    set_target_properties(gdcmDSED PROPERTIES INTERFACE_LINK_LIBRARIES "gdcmCommon")
    set_target_properties(gdcmIOD PROPERTIES INTERFACE_LINK_LIBRARIES "gdcmCommon;gdcmDICT;gdcmDSED")
    set_target_properties(gdcmMEXD PROPERTIES INTERFACE_LINK_LIBRARIES "gdcmCommon;gdcmDICT;gdcmDSED;gdcmIOD")
    set_target_properties(gdcmMSFF PROPERTIES INTERFACE_LINK_LIBRARIES "gdcmCommon;gdcmDICT;gdcmDSED;gdcmIOD;gdcmMEXD;gdcmcharls;gdcmexpat;gdcmjpeg8;gdcmjpeg12;gdcmjpeg16;gdcmopenjp2;gdcmuuid;jpeg;socketxx")

    include("${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib/imguiTargets.cmake")
    include("${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib/MRSymbolMeshTargets.cmake")
    include("${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib/MRVoxelsTargets.cmake")
    include("${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib/MRViewerTargets.cmake")
    include("${MESHLIB_SDK_ROOT}/lib/cmake/MeshLib/MRCommonPluginsTargets.cmake")
else()
    find_package(MeshLib REQUIRED)
endif()

include_directories(${MESHLIB_INCLUDE_DIR} ${MESHLIB_THIRDPARTY_INCLUDE_DIR})
link_directories(${MESHLIB_THIRDPARTY_LIB_DIR})

add_library(MeshInspectorWorkbenchPlugin MeshInspectorWorkbenchPlugin.cpp)
target_link_libraries(MeshInspectorWorkbenchPlugin
    PRIVATE
        MeshLib::MRViewer
)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${MESHLIB_EMSCRIPTEN_CXX_FLAGS}")

if(CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR)
    add_executable(ViewerApp ViewerApp.cpp)
    if(NOT EMSCRIPTEN)
        target_link_libraries(ViewerApp
            PRIVATE
                MeshLib::MRViewer
                MeshInspectorWorkbenchPlugin
        )
    else()
        target_link_libraries(ViewerApp
            PRIVATE
                -Wl,--whole-archive
                MeshLib::MRViewer
                MeshLib::MRIOExtras
                MeshLib::MRCommonPlugins
                MeshInspectorWorkbenchPlugin
                -Wl,--no-whole-archive
        )
        set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} ${MESHLIB_EMSCRIPTEN_EXE_LINKER_FLAGS}")

        file(GLOB MESHLIB_WASM_FILES "${MESHLIB_DATA_DIR}/wasm/*.*")
        file(COPY ${MESHLIB_WASM_FILES} DESTINATION ${HTML_DIR})
        file(COPY_FILE wasm/index.html ${HTML_DIR}/index.html)
        file(GLOB LOCAL_WASM_JS "${CMAKE_CURRENT_SOURCE_DIR}/wasm/*.js")
        if(LOCAL_WASM_JS)
            file(COPY ${LOCAL_WASM_JS} DESTINATION ${HTML_DIR})
        endif()

        file(GLOB MESHLIB_JSONS "${MESHLIB_DATA_DIR}/*.json")
        file(GLOB MESHLIB_FONTS "${MESHLIB_DATA_DIR}/fonts/*.*")
        file(GLOB LOCAL_JSONS   "${CMAKE_CURRENT_SOURCE_DIR}/*.json")
        file(
            COPY
                ${MESHLIB_DATA_DIR}/resource
                ${MESHLIB_JSONS}
                ${MESHLIB_FONTS}
                ${LOCAL_JSONS}
            DESTINATION "${HTML_DIR}/assets"
        )
        mr_emscripten_pack_directory("${HTML_DIR}/assets" "/")
        mr_emscripten_set_async_func_list("${MESHLIB_DATA_DIR}/wasm/wasm_async_func_list.txt")
    endif()
endif()
