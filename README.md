[![Build Status](https://api.travis-ci.org/alopatindev/lulzvm.svg?branch=master)](https://travis-ci.org/alopatindev/lulzvm)
[![Clippy Linting Result](https://clippy.bashy.io/github/alopatindev/lulzvm/master/badge.svg)](https://clippy.bashy.io/github/alopatindev/lulzvm/master/log)
[![Coverage Status](https://coveralls.io/repos/github/alopatindev/lulzvm/badge.svg?branch=master)](https://coveralls.io/github/alopatindev/lulzvm?branch=master)
[![codecov.io](http://codecov.io/github/alopatindev/lulzvm/coverage.svg?branch=master)](https://codecov.io/github/alopatindev/lulzvm?branch=master)
[![Language: Rust](https://img.shields.io/badge/language-Rust-orange.svg)](http://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.txt)

LulzVM
======

Byte-code interpreter for educational purposes.

Licensed under the terms of MIT (read LICENSE.txt for details).

## Design

### Features and Limitations
- 16-bit words, little-endian
- stack-based
    - locals stack
        - 8 bits per stack item
        - total size is 16 KiB
    - return address stack
        - 16 bits per stack item
        - total size is 2 KiB
- event queue
    - 16 bits per item (event id and data)
    - total size is 16 bytes
- unsigned integers arithmetic support only

### Executable Format
```
code size | code segment | data segment
```

### Memory Layout
```
executable | <-- locals stack | <-- return address stack | event handlers | <-- event queue
```

### Registers
|ID  |Title|Description         |
|----|-----|--------------------|
|0x00|PC   |Program Counter     |
|0x01|IR   |Instruction Register|
|0x02|SP   |Locals Stack Pointer|
|0x03|RP   |Return Stack Pointer|
|0x04|EP   |Event Queue Pointer |
|0x05|EE   |Event Queue End     |

### Flags
|Title                |Description                |
|---------------------|---------------------------|
|termination_scheduled|SIGINT/SIGTERM has happen  |
|terminated           |Finished program execution |
|waiting              |Waiting for any event      |

### Events
|ID  |Title           |Priority|
|----|----------------|--------|
|0x00|CLOCK           |Normal  |
|0x01|INPUT           |Normal  |
|0x02|OUTPUT          |Normal  |
|0x03|TERMINATE       |Fatal   |
|0x04|SEGFAULT        |Fatal   |
|0x05|UNKNOWN_ERROR   |Fatal   |

Fatal priority events run instantly.
