cmake_minimum_required(VERSION 3.28)

project(
    Shape
    VERSION 0.1.0
    DESCRIPTION "Intent-anchored cross-platform creative environment"
    LANGUAGES CXX
)

option(SHAPE_BUILD_DESKTOP "Build the Qt Quick desktop application" OFF)

include(CTest)

if(SHAPE_BUILD_DESKTOP)
    add_subdirectory(apps/desktop)
endif()

