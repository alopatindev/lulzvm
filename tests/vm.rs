extern crate lulzvm;

use lulzvm::utils;
use lulzvm::vm::events::*;
use lulzvm::vm::modes::*;
use lulzvm::vm::opcodes::*;

#[cfg_attr(rustfmt, rustfmt_skip)]
#[test]
fn simple() {
    {
        let executable = vec![0x00, 0x00];
        let (output, vm) = utils::test_run(&[], executable, 0);

        assert!(vm.data().is_empty());
        assert!(vm.locals_stack().is_empty());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert!(output.is_empty());
    }

    {
        let executable = vec![
            0x00, 0x00,

            NOP];
        let (output, vm) = utils::test_run(&[], executable, 0);

        assert!(vm.data().is_empty());
        assert!(vm.locals_stack().is_empty());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert!(output.is_empty());
    }

    {
        let executable = vec![
            0x00, 0x00,

            PUSH, 0x00,                    // b
            PUSH, 0x0a,                    // a
            DEC,                           // a--
            EMIT, OUTPUT,
            JNE, 0x06, 0x00];              // a != b

        let (output, vm) = utils::test_run(&[], executable, 0);

        assert!(vm.data().is_empty());
        assert_eq!([0x00, 0x00], vm.locals_stack());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert_eq!(&[9, 8, 7, 6, 5, 4, 3, 2, 1, 0], output.as_slice());
    }

    {
        let executable = vec![
            0x00, 0x00,

            PUSH, 0x00,                    // offset

            // label loop
            LOAD, PTR_WITH_OFFSET, 0x16, 0x00,
                                           // x = [data segment + offset]
            EMIT, OUTPUT,                  // print x
            PUSH, 0x00,                    // zero
            JE, 0x14, 0x00,                // if x == zero: goto end
            POP,                           // pop zero
            POP,                           // pop x
            INC,                           // offset++
            JMP, 0x04, 0x00,               // goto loop

            // label end
            NOP,                           // optional
            0x03, 0x02, 0x01, 0x00];

        let (output, vm) = utils::test_run(&[], executable, 4);

        assert_eq!(&[0x03, 0x02, 0x01, 0x00], vm.data());
        assert_eq!(&[0x00, 0x00, 0x03], vm.locals_stack());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert_eq!(&[0x03, 0x02, 0x01, 0x00], output.as_slice());
    }

    {
        let executable = vec![
            0x00, 0x00,

            PUSH, 0x00,                    // b

            // label loop
            EMIT, INPUT,                   // a
            EMIT, OUTPUT,                  // print a
            JE, 0x0f, 0x00,                // if a == b: goto end
            POP,                           // remove a
            JMP, 0x04, 0x00                // goto loop

            // label end
            // NOP                         // optional
            ];

        let input = [0x03, 0x02, 0x01, 0x00];
        let (output, vm) = utils::test_run(&input, executable, 0);

        assert!(vm.data().is_empty());
        assert_eq!(&[0x00, 0x00], vm.locals_stack());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert_eq!(&[0x03, 0x02, 0x01, 0x00], output.as_slice());
    }

    {
        let executable = vec![
            0x00, 0x00,

            PUSH, 0x00,                    // i
                                           // loop:
            LOAD, PTR, 0x15, 0x00,         // x
            JLE, 0x15, 0x00,               // if x <= i goto: end
            DEC,
            STORE, PTR, 0x15, 0x00,        // x
            POP,                           // pop x
            INC,                           // i++
            JMP, 0x04, 0x00,               // goto loop
                                           // end:
            0x05];                         // x

        let (output, vm) = utils::test_run(&[], executable, 1);

        assert_eq!(&[0x02], vm.data());
        assert_eq!(&[0x02, 0x03], vm.locals_stack());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert!(output.is_empty());
    }

    {
        let executable = vec![
            0x00, 0x00,

            PUSH, 0x00,                    // i
            PUSH, 0x05,                    // x
                                           // loop:
            DEC,
            SWP,
            INC,
            SWP,
            JLE, 0x10, 0,                  // if x <= i: goto end
            JMP, 0x06, 0];                 // goto loop

        let (output, vm) = utils::test_run(&[], executable, 0);

        assert!(vm.data().is_empty());
        assert_eq!(&[0x02, 0x03], vm.locals_stack());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert!(output.is_empty());
    }

    {
        let executable = vec![
            0x00, 0x00,

            PUSH, 0x00,                    // const zero
                                           // loop:
            EMIT, INPUT,                   //   x = read
            CALL, 0x14, 0x00,              //   x = f(x)
            EMIT, OUTPUT,                  //   print x
            JE, 0x12, 0x00,                // if x == zero: goto exit
            POP,                           // pop x
            JMP, 0x04, 0x00,               // goto loop
            EMIT, TERMINATE,               // exit:

                                           // f:
            PUSH, 0x02,
            MUL,                           // a = a * 2
            RET];

        let input = [0x03, 0x02, 0x01, 0x00];
        let (output, vm) = utils::test_run(&input, executable, 0);

        assert!(vm.data().is_empty());
        assert_eq!(&[0x00, 0x00], vm.locals_stack());
        assert!(vm.return_stack().is_empty());
        assert!(vm.event_queue().is_empty());
        assert_eq!(&[0x06, 0x04, 0x02, 0x00], output.as_slice());
    }
}
