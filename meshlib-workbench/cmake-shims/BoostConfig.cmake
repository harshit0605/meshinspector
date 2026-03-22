if(NOT TARGET Boost::headers)
  add_library(Boost::headers INTERFACE IMPORTED)
endif()

set(Boost_FOUND TRUE)
set(Boost_VERSION_STRING "1.87.0")
