cmake_minimum_required(VERSION 3.14)
project(DialecticaPlugin LANGUAGES CXX)

if(DEFINED ENV{LOGOS_MODULE_BUILDER_ROOT})
    include($ENV{LOGOS_MODULE_BUILDER_ROOT}/cmake/LogosModule.cmake)
else()
    message(FATAL_ERROR "LOGOS_MODULE_BUILDER_ROOT is not set.")
endif()

# One line, and that is the whole point of the Rust path (PLAN.md §2.2): there
# is no hand-written C++ here. The builder compiles rust-lib/ to a staticlib,
# generates the Qt glue from the .lidl it derived from the Rust trait, and links
# the two. No SOURCES, no INCLUDE_DIRS, no build.rs in the loop.
logos_module(NAME dialectica)
