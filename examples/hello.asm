.code
    ; code size is 0x13 bytes

    push 0x0d                        ; len(message)
    push 0x00

    loop:                            ; address 0x06
        jge exit
        load [message+offset]        ; load ptr_with_address 0x16 0x00
        emit output
        pop
        inc
        jmp loop

    exit:                            ; address 0x14
        emit terminate

.data
    message ascii "Hello World!\n"   ; address 0x16
