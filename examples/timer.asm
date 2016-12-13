.code
    ; code size is 0x12 bytes

    push 0x3a
    push 0x30

    loop:                            ; address 0x06
        jge restart
        emit output
        push 0x0a
        emit output
        pop
        inc
        wait
        jmp loop

    restart:                         ; address 0x15
        pop
        push 0x30
        jmp loop
