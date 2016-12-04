[![Build Status](https://api.travis-ci.org/alopatindev/lulzvm.svg?branch=master)](https://travis-ci.org/alopatindev/lulzvm)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.txt)

LulzVM
======

Byte-code interpreter for educational purposes.

Licensed under the terms of MIT (read LICENSE.txt for details).

## Design

### Features and Limitations
- 16-bit words, little-endian
- stack-based
    - one byte per stack item
    - stack size is 16 KiB
- unsigned integers arithmetic support only

### Executable Format
```
[ code size ] [ code segment ] [ data segment ]
```

### Memory Layout
```
[ executable ] [ registers ] [ <-- stack ] [ event handlers ] [ <-- event queue ]
```

### Event Queue
- 16 bits per item
    - event id
    - data

## Events
|ID  |Title           |Priority|
|----|----------------|--------|
|0x00|Not an item/end of queue||
|0x01|CLOCK           |Normal  |
|0x02|INPUT           |Normal  |
|0x03|OUTPUT          |Normal  |
|0xF0|TERMINATE       |Critical|
|0xF1|SEGFAULT        |Critical|
|0xF2|UNKNOWN_ERROR   |Critical|

Critical priority events run instantly, without queue.

### Registers
|ID  |Title |Description          |
|----|------|---------------------|
|0x00|PC    |Program Counter      |
|0x01|SP    |Stack Pointer        |
|0x02|IR    |Instruction Register |
|0x03|EP    |Event Queue Pointer  |
