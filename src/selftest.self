"A self-test of the language and the starting world, written in Self.
 Run it with:  selfTest run   (or World > Run the self-test)."

lobby _AddSlots: (|
  selfTest = (|
    {} = 'Checks that the Self engine and its world work. Send me run.'.
    parent* = traits clonable.
    passed <- 0.
    failed <- 0.
    failures <- nil.

    { 'Category: checking'
      check: name Gives: expected By: block = ( | got |
        got: [ block value ] onError: [| :m | 'error: ', m ].
        got = expected
          ifTrue: [ passed: passed + 1 ]
          False: [ failed: failed + 1.
                   failures add: name, ': got ', got printString, ', expected ', expected printString ].
        self ) }.

    { 'Category: helpers'
      firstEven: v = ( v do: [| :x | x even ifTrue: [ ^ x ] ]. nil ).
      countDown: n = ( n = 0 ifTrue: 0 False: [ countDown: n - 1 ] ) }.

    { 'Category: running'
      run = (
        passed: 0. failed: 0. failures: list copyRemoveAll.
        'Self World self-test' printLine.

        "numbers"
        check: 'addition' Gives: 7 By: [ 3 + 4 ].
        check: 'binary messages go left to right' Gives: 14 By: [ 3 + 4 * 2 ].
        check: 'unary before binary' Gives: 8 By: [ 2 + 3 factorial ].
        check: 'keyword after binary' Gives: 27 By: [ 3 + 4 factorial max: 20 ].
        check: 'integer division' Gives: 3 By: [ 7 / 2 ].
        check: 'float division' Gives: 3.5 By: [ 7 / 2.0 ].
        check: 'modulo' Gives: 2 By: [ -7 % 3 ].
        check: 'radix literal' Gives: 255 By: [ 16rFF ].
        check: 'big integers' Gives: '265252859812191058636308480000000' By: [ 30 factorial printString ].
        check: 'big integer arithmetic' Gives: 9900 By: [ 100 factorial / 98 factorial ].
        check: 'square root' Gives: 5.0 By: [ 25 sqrt ].
        check: 'primes' Gives: true By: [ 97 isPrime ].

        "booleans and nil"
        check: 'ifTrue:False:' Gives: 'yes' By: [ (3 > 2) ifTrue: [ 'yes' ] False: [ 'no' ] ].
        check: 'and / or' Gives: true By: [ (true && [ false not ]) || [ false ] ].
        check: 'nil' Gives: true By: [ nil isNil ].

        "strings"
        check: 'concatenation' Gives: 'hello world' By: [ 'hello', ' world' ].
        check: 'reverse and uppercase' Gives: 'CBA' By: [ 'abc' reverse asUppercase ].
        check: 'characters are strings' Gives: 'b' By: [ 'abc' at: 1 ].
        check: 'words' Gives: 3 By: [ 'the quick fox' words size ].

        "objects, slots and prototypes"
        check: 'object literal with a method' Gives: 7 By: [ (| x <- 3. y = 4. sum = ( x + y ) |) sum ].
        check: 'assignment slots' Gives: 9 By: [ | o | o: (| x <- 1 |). o x: 9. o x ].
        check: 'copies are separate' Gives: 70 By: [ | a | a: bankAccount copy. a deposit: 100. a withdraw: 30. a balance ].
        check: 'the prototype is untouched' Gives: 0 By: [ bankAccount balance ].
        check: 'points' Gives: '4@5' By: [ ((3 @ 4) + (1 @ 1)) printString ].
        check: 'point equality' Gives: true By: [ (3 @ 4) = (3 @ 4) ].
        check: 'adding slots' Gives: 2 By: [ | o | o: (| |) _Clone. o _AddSlots: (| n <- 1. bump = ( n: n + 1 ) |). o bump. o n ].
        check: 'reflection' Gives: '(\'parent\', \'x\', \'y\')' By: [ (reflect: 3 @ 4) names printString ].

        "inheritance"
        check: 'inherited method' Gives: 1 By: [ | o | o: (| parent* = list copyRemoveAll |). o add: 5. o size ].
        check: 'resend' Gives: 'a!' By: [ | p. c | p: (| parent* = traits clonable. say = 'a' |).
                                         c: (| parent* = p. say = ( resend.say, '!' ) |).
                                         c say ].
        check: 'dynamic inheritance' Gives: 'ab' By: [ | a. b. c. r |
                                         a: (| parent* = traits clonable. v = 'a' |).
                                         b: (| parent* = traits clonable. v = 'b' |).
                                         c: (| p* <- nil |).
                                         c p: a. r: c v. c p: b. r, c v ].

        "blocks"
        check: 'block with arguments' Gives: 42 By: [ [| :a. :b | a * b ] value: 6 With: 7 ].
        check: 'closures' Gives: 15 By: [ | adder | adder: [| :n | [| :x | x + n ] ]. (adder value: 10) value: 5 ].
        check: 'non-local return' Gives: 8 By: [ firstEven: (3 & 7 & 8 & 9) asVector ].
        check: 'loops' Gives: 5050 By: [ | r | r: 0. 1 to: 100 Do: [| :i | r: r + i ]. r ].
        check: 'whileTrue:' Gives: 10 By: [ | i | i: 0. [ i < 10 ] whileTrue: [ i: i + 1 ]. i ].
        check: 'recursion' Gives: 0 By: [ countDown: 150 ].

        "collections"
        check: 'vectors' Gives: '(1, 2, 3)' By: [ (1 & 2 & 3) asVector printString ].
        check: 'sorting' Gives: '(1, 4, 5)' By: [ (5 & 1 & 4) asVector sort printString ].
        check: 'collect and sum' Gives: 385 By: [ (1 to: 10 Collect: [| :i | i * i ]) sum ].
        check: 'select' Gives: 3 By: [ ((1 to: 6 Collect: [| :i | i ]) select: [| :x | x even ]) size ].
        check: 'inject:Into:' Gives: 24 By: [ (2 & 3 & 4) asVector inject: 1 Into: [| :a. :e | a * e ] ].
        check: 'lists grow' Gives: 100 By: [ | l | l: list copyRemoveAll. 1 to: 100 Do: [| :i | l add: i ]. l size ].
        check: 'dictionaries' Gives: 2 By: [ | d | d: dictionary copyRemoveAll. d at: 'a' Put: 1. d at: 'b' Put: 2. d at: 'b' ].

        "errors"
        check: 'message not understood' Gives: 'error: 3 does not understand \'foo\'' By: [ 3 foo ].
        check: 'primitive failure handler' Gives: 'divisionByZeroError' By: [ 1 _IntDiv: 0 IfFail: [| :e | e ] ].

        "morphs"
        check: 'morphs are objects' Gives: 'a circle morph' By: [ circleMorph copy printString ].
        check: 'adding a submorph' Gives: true By: [ | p. c | p: morph copy. c: labelMorph copy. p addMorph: c. c owner == p ].
        check: 'copies are deep' Gives: false By: [ | p | p: morph copy. p addMorph: labelMorph copy. p copy morphs first == p morphs first ].
        check: 'buttons run their action' Gives: 1 By: [ | n. b | n: 0. b: buttonMorph copyLabel: 'x' Action: [ n: n + 1 ]. b click. n ].
        check: 'deleting a morph' Gives: 0 By: [ | p. c | p: morph copy. c: morph copy. p addMorph: c. c delete. p morphs size ].

        "protocol carried over from the original core world"
        check: 'untilTrue: runs the body first' Gives: 5 By: [ | i | i: 0. [ i: i + 1 ] untilTrue: [ i >= 5 ]. i ].
        check: 'loopExit' Gives: 10 By: [ | i | i: 0. [| :exit | i: i + 1. i = 10 ifTrue: [ exit value ] ] loopExit. i ].
        check: 'rounding divisions' Gives: '4 3 3' By: [ (17 /+ 5) printString, ' ', (17 /- 5) printString, ' ', (17 /~ 5) printString ].
        check: 'compare:IfLess:Equal:Greater:' Gives: 'gt' By: [ 5 compare: 3 IfLess: 'lt' Equal: 'eq' Greater: 'gt' ].
        check: 'printStringPrecision:' Gives: '3.14' By: [ 3.14159 printStringPrecision: 2 ].
        check: 'factors' Gives: '(2, 2, 5, 5)' By: [ 100 factors printString ].
        check: 'doFirst:Middle:Last:IfEmpty:' Gives: 'F1M2L3' By: [ | r | r: ''.
          (1 & 2 & 3) asVector doFirst: [| :e | r: r, 'F', e printString ] Middle: [| :e | r: r, 'M', e printString ]
                                   Last: [| :e | r: r, 'L', e printString ] IfEmpty: [ r: 'E' ].
          r ].
        check: 'ifNone:IfOne:IfMany:' Gives: 'many' By: [ (7 & 8) asVector ifNone: 'none' IfOne: [| :e | e ] IfMany: 'many' ].
        check: 'joinUsing:' Gives: 'a, b, c' By: [ ('a' & 'b' & 'c') asVector joinUsing: ', ' ].
        check: 'reduceWith:' Gives: 24 By: [ (1 & 2 & 3 & 4) asVector reduceWith: [| :a. :b | a * b ] ].
        check: 'stable merge sort' Gives: '(\'b1\', \'a1\', \'a2\', \'b2\')' By: [
          (('b1' & 'a2' & 'b2' & 'a1') asVector copySortBy: [| :x. :y | (x at: 1) < (y at: 1) ]) printString ].
        check: 'sets keep one of each' Gives: 'set(1, 2, 3)' By: [ (1 & 2 & 2 & 3 & 1) asVector asSet printString ].
        check: 'intersect:' Gives: '(2, 4)' By: [ ((1 & 2 & 3 & 4) asVector intersect: (2 & 4 & 6) asVector) printString ].
        check: 'lists are indexable' Gives: 'list(2, 3)' By: [ ((1 & 2 & 3 & 4) asList copyFrom: 1 UpTo: 3) printString ].
        check: 'addFirst:' Gives: 'list(0, 1, 2)' By: [ | l | l: (1 & 2) asList. l addFirst: 0. l printString ].
        check: 'at:IfAbsentPut:' Gives: 'one' By: [ | d | d: dictionary copyRemoveAll.
          d at: 1 IfAbsentPut: [ 'one' ]. d at: 1 IfAbsentPut: [ 'uno' ] ].
        check: 'string prefixes' Gives: 'hello' By: [ 'hello.self' copyWithoutSuffix: '.self' ].
        check: 'pattern matching' Gives: true By: [ 'world.self' matchesPattern: 'w*.s?lf' ].
        check: 'tokens' Gives: 3 By: [ '  one two\tthree ' asTokensSeparatedByWhiteSpace size ].
        check: 'sendTo:With:' Gives: 7 By: [ '+' sendTo: 3 With: 4 ].
        check: 'point arithmetic with numbers' Gives: '4@5' By: [ ((3 @ 4) + 1) printString ].
        check: 'rectangles' Gives: '(5@5) # (10@10)' By: [ (((0 @ 0) # (10 @ 10)) intersect: ((5 @ 5) # (20 @ 20))) printString ].
        check: 'rectangle area' Gives: 24 By: [ ((0 @ 0) ## (4 @ 6)) area ].

        "unwind protection, files and saving objects"
        check: 'onReturn: runs on a normal return' Gives: 11 By: [ | r | r: 0. [ r: 1 ] onReturn: [ r: r + 10 ]. r ].
        check: 'onReturn: runs on an error too' Gives: 'cleaned' By: [ | r | [ [ 1 / 0 ] onReturn: [ r: 'cleaned' ] ] onError: [ ]. r ].
        check: 'loopExitContinue' Gives: '135' By: [ | n. log | n: 0. log: ''.
          [| :exit. :continue | n: n + 1. n > 6 ifTrue: [ exit value ]. n even ifTrue: [ continue value ]. log: log, n printString ] loopExitContinue.
          log ].
        check: 'vectors compare their elements' Gives: true By: [ (1 & 2) asVector = (1 & 2) asVector ].
        check: 'lines' Gives: 2 By: [ ('a b', '\n', 'c d') lines size ].
        check: 'storeString rebuilds an object' Gives: 7 By: [ ((| parent* = traits clonable. x <- 3. y = 4. sum = ( x + y ) |) storeString eval) sum ].
        check: 'fileOut changes' Gives: true By: [ fileOut changes includesSubstring: 'Objects saved from Self World' ].

        "morphs for building apps"
        check: 'list morphs select' Gives: 'b' By: [ | l | l: listMorph copyItems: ('a' & 'b') asVector. l select: 1. l selection ].
        check: 'drop-downs start on their first choice' Gives: 'x' By: [ (dropDownMorph copyChoices: ('x' & 'y') asVector) selected ].
        check: 'clicks with a position' Gives: '3@4' By: [ | m | m: canvasMorph copy.
          m _AddSlots: (| got <- nil. clickAt: p = ( got: p ) |). m clickAtX: 3 Y: 4. m got printString ].

        "original Self file format"
        check: 'bootstrap paths' Gives: true By: [ (bootstrap stub -> 'globals' -> 'true' -> ()) == true ].
        check: 'eval' Gives: 7 By: [ '3 + 4' eval ].

        report ) }.

    { 'Category: printing'
      report = (
        failures do: [| :f | ('  FAILED  ', f) printLine ].
        (passed printString, ' passed, ', failed printString, ' failed') printLine.
        failed = 0
          ifTrue: [ 'All ', passed printString, ' checks passed' ]
          False: [ failed printString, ' of ', (passed + failed) printString, ' checks failed (see the transcript)' ] ) }
  |)
|).
