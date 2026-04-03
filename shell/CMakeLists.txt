#[[
    Unix V6++ Shell 构造文件
    创建于 2024年4月28日 上海市嘉定区安亭镇

    by 2051565 GTY
]]


cmake_minimum_required(VERSION 3.20)


#[[ 
    项目构造产物
]]
file(GLOB_RECURSE C_SOURCE_FILES *.c)
set(CMAKE_CXX_LINK_EXECUTABLE "ld --whole-archive -Ttext-segment=0x400000 <LINK_FLAGS> -e main1 -static <LINK_LIBRARIES> <OBJECTS> -o <TARGET>")
set(CMAKE_C_LINK_EXECUTABLE "${CMAKE_CXX_LINK_EXECUTABLE}")


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

    set(ASM_DUMP_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../target/asm-dump")
    file(MAKE_DIRECTORY ${ASM_DUMP_DIR})

    if (0)  # optional
        add_custom_command(
            TARGET ${taskName} POST_BUILD
            COMMAND
                objdump -D ${binDir}/${exeName} > ${ASM_DUMP_DIR}/${taskName}.full.asm
        )
    endif()
endfunction(addExe)


addExe(Shell.elf.exe "${C_SOURCE_FILES}" "elf_i386" "${CMAKE_CURRENT_SOURCE_DIR}/../target/objs")
addExe(Shell.exe "${C_SOURCE_FILES}" "i386pe" "${CMAKE_CURRENT_SOURCE_DIR}/../target/objs")

