#[[
    Unix V6++ apps 构造文件
    创建于 2024年4月28日 上海市嘉定区安亭镇

    by 2051565 GTY
]]



cmake_minimum_required(VERSION 3.20)
project(apps C) #erase warning on lower cmake version

# GCC(g++) 编译选项。
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -e main1")


#[[ 
    项目构造产物
]]
file(GLOB_RECURSE C_SOURCE_FILES *.c)

#[[
    
    params:
      exeFormat: elf_i386 or i386pe
]]
function(addExe exeName src exeFormat binDir)

    set(taskName "${exeName}-${exeFormat}")
    add_executable(${taskName} ${src})
    target_link_libraries(${taskName} ${CMAKE_SOURCE_DIR}/../target/objs/libv6pptongji.a)

    set_target_properties(
        ${taskName} 
        PROPERTIES 
            COMPILE_FLAGS "-m32" 
            LINK_FLAGS "-m32 -m ${exeFormat}"
    )
    
    add_custom_command(
        TARGET ${taskName} POST_BUILD
        COMMAND
            ${CMAKE_COMMAND} -E copy 
                $<TARGET_FILE:${taskName}> 
                ${binDir}/${exeName}
    )

    set(ASM_DUMP_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../target/asm-dump/apps")
    file(MAKE_DIRECTORY ${ASM_DUMP_DIR})

    if (0)  # optional
        add_custom_command(
            TARGET ${taskName} POST_BUILD
            COMMAND
                objdump -D ${binDir}/${exeName} > ${ASM_DUMP_DIR}/${taskName}.full.asm
        )
    endif()
endfunction(addExe)


foreach(src IN LISTS C_SOURCE_FILES)
    string(REGEX REPLACE "[.]c$" ".exe" exe ${src})

    get_filename_component(exeName ${exe} NAME_WE)
    addExe(${exeName} ${src} "elf_i386" "${CMAKE_CURRENT_SOURCE_DIR}/../target/objs/apps-elf")
    addExe(${exeName} ${src} "i386pe" "${CMAKE_CURRENT_SOURCE_DIR}/../target/objs/apps")


endforeach()


