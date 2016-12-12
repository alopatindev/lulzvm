.code
    ; code size is 0x13 bytes

    push 0x0d                        ; len(message)
    push 0x00

    loop:                            ; address 0x06
        jge exit
        load_offs [message]          ; load_offs 0x15 0x00
        emit output
        pop
        inc
        jmp loop

    exit:                            ; address 0x13
        emit terminate

.data
    message ascii "Hello World!\n"   ; address 0x15
