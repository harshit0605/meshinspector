if(NOT DEFINED MESHLIB_SDK_ROOT)
  if(DEFINED CMAKE_FIND_ROOT_PATH AND NOT "${CMAKE_FIND_ROOT_PATH}" STREQUAL "")
    list(GET CMAKE_FIND_ROOT_PATH 0 MESHLIB_SDK_ROOT)
  else()
    message(FATAL_ERROR "MESHLIB_SDK_ROOT or CMAKE_FIND_ROOT_PATH must point to the extracted MeshLib SDK root")
  endif()
endif()

if(NOT TARGET Eigen3::Eigen)
  add_library(Eigen3::Eigen INTERFACE IMPORTED)
  set_target_properties(Eigen3::Eigen PROPERTIES
    INTERFACE_INCLUDE_DIRECTORIES "${MESHLIB_SDK_ROOT}/include/eigen3"
  )
endif()

set(Eigen3_FOUND TRUE)
