# Read by whisper.cpp's cmake after each `project()` (CMAKE_PROJECT_INCLUDE, in config.toml).
#
# With MSVC, the cmake crate sets CMAKE_<LANG>_FLAGS_<BUILD TYPE> to cc's flags with every /O… taken out ("let cmake
# deal with optimization"), which leaves cmake none: whisper.cpp was built unoptimized, a sentence taking a minute with
# large-v3-turbo (ESC-135). Its optimizations given back, as cmake's own defaults have them. Nothing elsewhere.
# Cargo does not build whisper-rs-sys again when this file changes: `cargo clean -p whisper-rs-sys` (and `--release`)
# on a target folder already built. The Release does it on Windows each time (release.yml).
if(MSVC)
  foreach(lang C CXX)
    if(NOT CMAKE_${lang}_FLAGS_RELEASE MATCHES "[-/]O[12x]")
      string(APPEND CMAKE_${lang}_FLAGS_RELEASE " /O2 /Ob2 /DNDEBUG")
    endif()
    if(NOT CMAKE_${lang}_FLAGS_RELWITHDEBINFO MATCHES "[-/]O[12x]")
      string(APPEND CMAKE_${lang}_FLAGS_RELWITHDEBINFO " /O2 /Ob1 /DNDEBUG")
    endif()
  endforeach()
endif()
