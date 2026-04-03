#[[
    Lib 构造文件
    创建于 2024年4月28日 上海市嘉定区安亭镇

    by 2051565 GTY
]]

cmake_minimum_required(VERSION 3.20)

project(v6pptongji)

#[[ 
    项目构造产物
]]
file(GLOB_RECURSE CPP_SOURCE_FILES *.cpp)
file(GLOB_RECURSE C_SOURCE_FILES *.c)
add_library(
    v6pptongji STATIC
    
    ${CPP_SOURCE_FILES}
    ${C_SOURCE_FILES}
)
set_target_properties(
    v6pptongji PROPERTIES 
    COMPILE_FLAGS "-m32" 
    LINK_FLAGS "-m32"
)


