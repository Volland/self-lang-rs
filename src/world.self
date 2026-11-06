"The starting world for Self World, written in Self.

 Like the original Self system, almost all behaviour lives in objects:
 the lobby inherits from globals and defaultBehavior, traits objects hold
 shared behaviour, and prototypes are copied to make new objects. Only the
 primitives (messages starting with an underscore) are built into the VM."

defaultBehavior _AddSlots: (|
  {} = 'The default behaviour for the lobby and every object that inherits from it.'.
  { 'Category: comparing'
    == x = ( _Eq: x ).
    !== x = ( (_Eq: x) not ).
    = x = ( _Eq: x ).
    != x = ( (self = x) not ).
    hash = ( _IdentityHash ).
    identityHash = ( _IdentityHash ) }.
  { 'Category: printing'
    printString = ( _ObjectName ).
    print = ( printString _StringPrint. self ).
    printLine = ( printString _StringPrintLine. self ).
    asString = ( printString ).
    name = ( _ObjectName ) }.
  { 'Category: testing'
    isNil = false.
    notNil = true.
    ifNil: b = ( self ).
    ifNotNil: b = ( b value: self ).
    ifNil: nb IfNotNil: b = ( b value: self ).
    isNumber = false.
    isString = false.
    isVector = false.
    isBlock = false.
    isBoolean = false }.
  { 'Category: evaluating'
    value = ( self ).
    yourself = ( self ) }.
  { 'Category: collecting'
    & x = ( (list copyRemoveAll add: self) add: x ) }.
  { 'Category: errors'
    error: msg = ( _Error: msg ).
    halt = ( _Error: 'halt' ) }.
  { 'Category: reflection'
    inspect = ( _Inspect ).
    reflect: x = ( mirror copy reflectee: x ).
    slotNames = ( _SlotNames ).
    kindName = ( _KindName ).
    perform: sel = ( _Perform: sel ).
    perform: sel With: a = ( _Perform: sel With: a ).
    perform: sel With: a With: b = ( _Perform: sel With: a With: b ) }
|).

traits _AddSlots: (|
  clonable = (|
    {} = 'Objects that may be copied: most prototypes inherit from me.'.
    parent* = lobby.
    copy = ( _Clone ).
    clone = ( _Clone ) |).
  oddball = (|
    {} = 'Unique objects such as nil, true and numbers: copying returns the object itself.'.
    parent* = lobby.
    copy = ( self ).
    clone = ( self ) |)
|).

traits _AddSlots: (|
  boolean = (| parent* = traits oddball. isBoolean = true |).
  number = (| parent* = traits oddball |).
  collection = (| parent* = traits clonable |)
|).

traits _AddSlots: (|
  indexable = (| parent* = traits collection |)
|).

"---------------------------------------------------------------- booleans and nil"

true _AddSlots: (|
  parent* = traits boolean.
  { 'Category: control'
    ifTrue: b = ( b value ).
    ifFalse: b = ( nil ).
    ifTrue: a False: b = ( a value ).
    ifFalse: a True: b = ( b value ) }.
  { 'Category: logic'
    && b = ( b value ).
    || b = ( true ).
    and: b = ( b value ).
    or: b = ( true ).
    ^^ b = ( b value not ).
    not = false }.
  printString = 'true'.
  asInteger = 1
|).

false _AddSlots: (|
  parent* = traits boolean.
  { 'Category: control'
    ifTrue: b = ( nil ).
    ifFalse: b = ( b value ).
    ifTrue: a False: b = ( b value ).
    ifFalse: a True: b = ( a value ) }.
  { 'Category: logic'
    && b = ( false ).
    || b = ( b value ).
    and: b = ( false ).
    or: b = ( b value ).
    ^^ b = ( b value ).
    not = true }.
  printString = 'false'.
  asInteger = 0
|).

nil _AddSlots: (|
  {} = 'The object that slots hold before anything else is put in them.'.
  parent* = traits oddball.
  isNil = true.
  notNil = false.
  ifNil: b = ( b value ).
  ifNotNil: b = ( nil ).
  ifNil: b IfNotNil: nb = ( b value ).
  printString = 'nil'
|).

"---------------------------------------------------------------- numbers"

traits number _AddSlots: (|
  { 'Category: testing'
    isNumber = true.
    isZero = ( self = 0 ).
    sign = ( self > 0 ifTrue: 1 False: [ self < 0 ifTrue: -1 False: 0 ] ) }.
  { 'Category: comparing'
    max: x = ( self > x ifTrue: self False: x ).
    min: x = ( self < x ifTrue: self False: x ).
    between: a And: b = ( (self >= a) && [ self <= b ] ).
    != x = ( (self = x) not ) }.
  { 'Category: arithmetic'
    negated = ( 0 - self ).
    abs = ( self < 0 ifTrue: [ self negated ] False: self ).
    squared = ( self * self ).
    sqrt = ( asFloat sqrt ).
    pi = 3.141592653589793 }.
  { 'Category: geometry'
    @ y = ( (point copy x: self) y: y ) }
|).

traits integer _AddSlots: (|
  {} = 'Shared behaviour of all integers, small and big. Mixing in a float gives a float.'.
  parent* = traits number.
  { 'Category: arithmetic'
    + a = ( _IntAdd: a IfFail: [ asFloat + a ] ).
    - a = ( _IntSub: a IfFail: [ asFloat - a ] ).
    * a = ( _IntMul: a IfFail: [ asFloat * a ] ).
    "Integers never overflow: they grow into big integers, as in the original Self."
    / a = ( _IntDiv: a IfFail: [| :e | e = 'divisionByZeroError'
                                   ifTrue: [ error: 'division by zero' ]
                                   False: [ asFloat / a ] ] ).
    % a = ( _IntMod: a IfFail: [ error: 'division by zero' ] ).
    rem: a = ( _IntRem: a IfFail: [ error: 'division by zero' ] ).
    succ = ( self + 1 ).
    pred = ( self - 1 ).
    raisedTo: n = ( | r <- 1 | n timesRepeat: [ r: r * self ]. r ).
    factorial = ( | r <- 1 | self < 0 ifTrue: [ ^ error: 'factorial of a negative number' ].
      2 to: self Do: [| :i | r: r * i ]. r ).
    fibonacci = ( self < 2 ifTrue: self False: [ (self - 1) fibonacci + (self - 2) fibonacci ] ).
    gcd: n = ( n = 0 ifTrue: [ self abs ] False: [ n gcd: self % n ] ).
    lcm: n = ( (self * n) abs / (gcd: n) ) }.
  { 'Category: comparing'
    < a = ( _IntLT: a IfFail: [ asFloat < a ] ).
    <= a = ( _IntLE: a IfFail: [ asFloat <= a ] ).
    > a = ( _IntGT: a IfFail: [ asFloat > a ] ).
    >= a = ( _IntGE: a IfFail: [ asFloat >= a ] ).
    = a = ( _IntEQ: a IfFail: [ a isNumber ifTrue: [ asFloat = a ] False: false ] ).
    hash = ( self ) }.
  { 'Category: testing'
    even = ( (self % 2) = 0 ).
    odd = ( even not ).
    isPrime = ( | d <- 2 |
      self < 2 ifTrue: [ ^ false ].
      [ (d * d) <= self ] whileTrue: [
        (self % d) = 0 ifTrue: [ ^ false ].
        d: d + 1 ].
      true ) }.
  { 'Category: bits'
    bitAnd: a = ( _IntAnd: a ).
    bitOr: a = ( _IntOr: a ).
    bitXor: a = ( _IntXor: a ).
    << n = ( _IntArithmeticShiftLeft: n ).
    >> n = ( _IntArithmeticShiftRight: n ) }.
  { 'Category: iterating'
    to: e Do: b = ( | i | i: self. [ i <= e ] whileTrue: [ b value: i. i: i + 1 ]. self ).
    to: e By: s Do: b = ( | i |
      i: self.
      s > 0 ifTrue: [ [ i <= e ] whileTrue: [ b value: i. i: i + s ] ]
            False: [ [ i >= e ] whileTrue: [ b value: i. i: i + s ] ].
      self ).
    upTo: e Do: b = ( | i | i: self. [ i < e ] whileTrue: [ b value: i. i: i + 1 ]. self ).
    downTo: e Do: b = ( | i | i: self. [ i >= e ] whileTrue: [ b value: i. i: i - 1 ]. self ).
    timesRepeat: b = ( | i <- 0 | [ i < self ] whileTrue: [ b value. i: i + 1 ]. self ).
    to: e Collect: b = ( | v | v: vector copySize: ((e - self) + 1) max: 0.
      to: e Do: [| :i | v at: i - self Put: (b value: i) ].
      v ) }.
  { 'Category: converting'
    asFloat = ( _IntAsFloat ).
    asInteger = ( self ).
    truncate = ( self ).
    rounded = ( self ).
    asCharacter = ( _IntAsCharacter ).
    random = ( _Random: self ) }.
  { 'Category: printing'
    printString = ( _IntPrintString ).
    printStringBase: b = ( _IntPrintStringBase: b ).
    hexPrintString = ( _IntPrintStringBase: 16 ) }
|).

traits float _AddSlots: (|
  {} = 'Shared behaviour of all floating point numbers.'.
  parent* = traits number.
  { 'Category: arithmetic'
    + a = ( _FloatAdd: a ).
    - a = ( _FloatSub: a ).
    * a = ( _FloatMul: a ).
    / a = ( _FloatDiv: a IfFail: [ error: 'division by zero' ] ).
    % a = ( _FloatMod: a ).
    raisedTo: n = ( _FloatPow: n ).
    sqrt = ( _FloatSqrt ).
    sin = ( _FloatSin ).
    cos = ( _FloatCos ).
    tan = ( _FloatTan ).
    arcTan = ( _FloatArcTan ).
    ln = ( _FloatLn ).
    exp = ( _FloatExp ) }.
  { 'Category: comparing'
    < a = ( _FloatLT: a ).
    <= a = ( _FloatLE: a ).
    > a = ( _FloatGT: a ).
    >= a = ( _FloatGE: a ).
    = a = ( _FloatEQ: a IfFail: false ) }.
  { 'Category: converting'
    asFloat = ( self ).
    asInteger = ( _FloatAsInt ).
    truncate = ( _FloatTruncate ).
    rounded = ( _FloatRound ).
    floor = ( _FloatFloor ).
    ceil = ( _FloatCeil ) }.
  printString = ( _FloatPrintString )
|).

"---------------------------------------------------------------- collections"

traits collection _AddSlots: (|
  {} = 'Behaviour shared by every collection. Children supply size and do:.'.
  { 'Category: testing'
    isEmpty = ( size = 0 ).
    notEmpty = ( size > 0 ).
    includes: x = ( do: [| :e | e = x ifTrue: [ ^ true ] ]. false ).
    indexOf: x = ( do: [| :e. :i | e = x ifTrue: [ ^ i ] ]. -1 ) }.
  { 'Category: enumerating'
    inject: acc Into: b = ( | r | r: acc. do: [| :e | r: (b value: r With: e) ]. r ).
    detect: b = ( detect: b IfNone: nil ).
    detect: b IfNone: nb = ( do: [| :e | (b value: e) ifTrue: [ ^ e ] ]. nb value ).
    count: b = ( | n <- 0 | do: [| :e | (b value: e) ifTrue: [ n: n + 1 ] ]. n ).
    anySatisfy: b = ( do: [| :e | (b value: e) ifTrue: [ ^ true ] ]. false ).
    allSatisfy: b = ( do: [| :e | (b value: e) ifFalse: [ ^ false ] ]. true ).
    select: b = ( | l | l: list copyRemoveAll. do: [| :e | (b value: e) ifTrue: [ l add: e ] ]. l asVector ).
    reject: b = ( select: [| :e | (b value: e) not ] ).
    collect: b = ( | l | l: list copyRemoveAll. do: [| :e | l add: (b value: e) ]. l asVector ) }.
  { 'Category: math'
    sum = ( inject: 0 Into: [| :a. :e | a + e ] ).
    max = ( | m | do: [| :e | (m isNil || [ e > m ]) ifTrue: [ m: e ] ]. m ).
    min = ( | m | do: [| :e | (m isNil || [ e < m ]) ifTrue: [ m: e ] ]. m ).
    average = ( sum asFloat / size ) }.
  { 'Category: converting'
    asVector = ( | v. i <- 0 | v: vector copySize: size. do: [| :e | v at: i Put: e. i: i + 1 ]. v ).
    asList = ( | l | l: list copyRemoveAll. do: [| :e | l add: e ]. l ) }
|).

traits indexable _AddSlots: (|
  {} = 'Collections whose elements are numbered from 0, such as vectors and strings.'.
  { 'Category: enumerating'
    do: b = ( 0 upTo: size Do: [| :i | b value: (at: i) With: i ]. self ).
    reverseDo: b = ( (size - 1) downTo: 0 Do: [| :i | b value: (at: i) With: i ]. self ).
    collect: b = ( | r | r: vector copySize: size.
      0 upTo: size Do: [| :i | r at: i Put: (b value: (at: i) With: i) ].
      r ).
    with: other Do: b = ( 0 upTo: size Do: [| :i | b value: (at: i) With: (other at: i) ]. self ) }.
  { 'Category: accessing'
    first = ( at: 0 ).
    last = ( at: size - 1 ) }
|).

traits _AddSlots: (|
  vector = (| parent* = traits indexable |)
|).

globals _AddSlots: (|
  vector = (| parent* = traits vector |) _MakeVector
|).

traits vector _AddSlots: (|
  {} = 'Fixed-size arrays of any objects. Make new ones with: vector copySize: 3'.
  { 'Category: accessing'
    size = ( _Size ).
    at: i = ( _At: i IfFail: [ error: 'index ', i printString, ' is outside 0 to ', (size - 1) printString ] ).
    at: i Put: x = ( _At: i Put: x IfFail: [ error: 'index ', i printString, ' is outside 0 to ', (size - 1) printString ] ) }.
  { 'Category: copying'
    copySize: n = ( _Clone: n Filler: nil ).
    copySize: n FillingWith: x = ( _Clone: n Filler: x ).
    copyAddLast: x = ( (copySize: size + 1) at: size Put: x ).
    , v = ( | r | r: (copySize: size + v size).
      0 upTo: v size Do: [| :i | r at: size + i Put: (v at: i) ].
      r ).
    reverse = ( | r | r: (copySize: size).
      0 upTo: size Do: [| :i | r at: (size - 1) - i Put: (at: i) ].
      r ).
    sort = ( sortBy: [| :a. :b | a < b ] ).
    sortBy: lessThan = ( | r. j. x |
      r: copy.
      1 upTo: r size Do: [| :i |
        x: r at: i.
        j: i - 1.
        [ (j >= 0) && [ lessThan value: x With: (r at: j) ] ] whileTrue: [
          r at: j + 1 Put: (r at: j).
          j: j - 1 ].
        r at: j + 1 Put: x ].
      r ) }.
  isVector = true.
  printString = ( | s <- '' |
    do: [| :e. :i | i > 0 ifTrue: [ s: s, ', ' ]. s: s, e printString ].
    '(', s, ')' )
|).

traits _AddSlots: (|
  list = (| parent* = traits collection |)
|).

globals _AddSlots: (|
  list = (| parent* = traits list. items <- vector copySize: 4. count <- 0 |)
|).

traits list _AddSlots: (|
  {} = 'A collection that grows as you add to it: list copyRemoveAll add: 3'.
  { 'Category: adding'
    add: x = (
      count = items size ifTrue: [ items: items copySize: (items size * 2) max: 4 ].
      items at: count Put: x.
      count: count + 1.
      self ).
    addLast: x = ( add: x ).
    addAll: c = ( c do: [| :e | add: e ]. self ).
    & x = ( add: x ) }.
  { 'Category: removing'
    removeLast = ( | x | isEmpty ifTrue: [ ^ error: 'the list is empty' ].
      count: count - 1. x: items at: count. items at: count Put: nil. x ).
    removeFirst = ( | x | isEmpty ifTrue: [ ^ error: 'the list is empty' ].
      x: items at: 0.
      1 upTo: count Do: [| :i | items at: i - 1 Put: (items at: i) ].
      count: count - 1.
      x ).
    removeAll = ( count: 0. items: vector copySize: 4. self ) }.
  { 'Category: accessing'
    size = ( count ).
    at: i = ( ((i < 0) || [ i >= count ]) ifTrue: [ ^ error: 'index ', i printString, ' is outside the list' ].
      items at: i ).
    at: i Put: x = ( ((i < 0) || [ i >= count ]) ifTrue: [ ^ error: 'index ', i printString, ' is outside the list' ].
      items at: i Put: x. self ).
    first = ( at: 0 ).
    last = ( at: count - 1 ) }.
  { 'Category: enumerating'
    do: b = ( 0 upTo: count Do: [| :i | b value: (items at: i) With: i ]. self ) }.
  { 'Category: copying'
    copy = ( resend.copy items: items copy ).
    copyRemoveAll = ( copy removeAll ).
    asVector = ( items copySize: count ) }.
  printString = ( 'list', asVector printString )
|).

traits _AddSlots: (|
  dictionary = (| parent* = traits collection |)
|).

globals _AddSlots: (|
  dictionary = (| parent* = traits dictionary. keys <- list copyRemoveAll. values <- list copyRemoveAll |)
|).

traits dictionary _AddSlots: (|
  {} = 'Keys mapped to values: dictionary copyRemoveAll at: 1 Put: 2'.
  { 'Category: accessing'
    at: k Put: v = ( | i | i: keys indexOf: k.
      i < 0 ifTrue: [ keys add: k. values add: v ] False: [ values at: i Put: v ].
      self ).
    at: k = ( at: k IfAbsent: [ error: 'key not found: ', k printString ] ).
    at: k IfAbsent: b = ( | i | i: keys indexOf: k. i < 0 ifTrue: [ b value ] False: [ values at: i ] ).
    includesKey: k = ( (keys indexOf: k) >= 0 ).
    removeKey: k = ( | i. nk. nv | i: keys indexOf: k.
      i < 0 ifTrue: [ ^ self ].
      nk: list copyRemoveAll. nv: list copyRemoveAll.
      keys do: [| :e. :j | j = i ifFalse: [ nk add: e. nv add: (values at: j) ] ].
      keys: nk. values: nv. self ).
    size = ( keys size ) }.
  { 'Category: enumerating'
    do: b = ( values do: [| :v. :i | b value: v With: (keys at: i) ]. self ).
    keysDo: b = ( keys do: b. self ) }.
  { 'Category: copying'
    copy = ( (resend.copy keys: keys copy) values: values copy ).
    copyRemoveAll = ( (resend.copy keys: list copyRemoveAll) values: list copyRemoveAll ) }.
  printString = ( | s <- '' |
    keys do: [| :k. :i | i > 0 ifTrue: [ s: s, ', ' ]. s: s, k printString, ' -> ', (values at: i) printString ].
    'dictionary(', s, ')' )
|).

"---------------------------------------------------------------- strings"

traits string _AddSlots: (|
  {} = 'Immutable strings. Characters are strings of length one; indices start at 0.'.
  parent* = traits indexable.
  { 'Category: accessing'
    size = ( _StringSize ).
    at: i = ( _StringAt: i IfFail: [ error: 'index ', i printString, ' is outside the string' ] ).
    byteAt: i = ( _StringByteAt: i ).
    asByte = ( _StringByteAt: 0 ) }.
  { 'Category: comparing'
    = s = ( _StringEQ: s ).
    < s = ( _StringLT: s ).
    > s = ( s < self ).
    hash = ( _StringHash ) }.
  { 'Category: copying'
    , s = ( _StringConcat: s asString ).
    copyFrom: a UpTo: b = ( _StringCopyFrom: a UpTo: b IfFail: [ error: 'bad range ', a printString, ' to ', b printString ] ).
    copyFrom: a = ( copyFrom: a UpTo: size ).
    reverse = ( _StringReverse ).
    asUppercase = ( _StringAsUppercase ).
    asLowercase = ( _StringAsLowercase ).
    trim = ( _StringTrim ).
    capitalize = ( isEmpty ifTrue: self False: [ (copyFrom: 0 UpTo: 1) asUppercase, (copyFrom: 1) ] ) }.
  { 'Category: searching'
    includesSubstring: s = ( (_StringIndexOf: s) >= 0 ).
    indexOfSubstring: s = ( _StringIndexOf: s ).
    split: sep = ( _StringSplit: sep ).
    words = ( _StringSplit: ' ' ).
    lines = ( _StringSplit: '\n' ) }.
  { 'Category: converting'
    asString = ( self ).
    asInteger = ( _StringAsIntegerIfFail: [ error: 'not an integer: ', self printString ] ).
    asFloat = ( _StringAsFloatIfFail: [ error: 'not a number: ', self printString ] ).
    asNumber = ( _StringAsIntegerIfFail: [ asFloat ] ) }.
  { 'Category: printing'
    printString = ( _StringPrintString ).
    print = ( _StringPrint ).
    printLine = ( _StringPrintLine ) }.
  { 'Category: evaluating'
    eval = ( _StringEval ) }.
  isString = true
|).

"---------------------------------------------------------------- blocks"

traits block _AddSlots: (|
  {} = 'Blocks are closures: [| :x | x * 2 ] value: 21'.
  parent* = traits oddball.
  { 'Category: looping'
    whileTrue: b = ( _WhileTrue: b ).
    whileFalse: b = ( _WhileFalse: b ).
    whileTrue = ( _WhileTrue: [] ).
    whileFalse = ( _WhileFalse: [] ).
    untilTrue: b = ( _WhileFalse: b ).
    loop = ( _Loop ) }.
  { 'Category: evaluating'
    numArgs = ( _BlockNumArgs ).
    valueWithArguments: v = ( _ValueWithArguments: v ).
    onError: handler = ( _OnError: handler ).
    msecToRun = ( | t | t: _TimeMsec. value. _TimeMsec - t ) }.
  isBlock = true.
  printString = 'a block'
|).

traits method _AddSlots: (|
  parent* = traits oddball.
  printString = 'a method'
|).

"---------------------------------------------------------------- points, mirrors, utilities"

traits _AddSlots: (|
  point = (| parent* = traits clonable |).
  mirror = (| parent* = traits clonable |)
|).

globals _AddSlots: (|
  point = (| parent* = traits point. x <- 0. y <- 0 |).
  mirror = (| parent* = traits mirror. reflectee <- nil |)
|).

traits point _AddSlots: (|
  {} = 'Cartesian points. Make one with 3 @ 4'.
  { 'Category: arithmetic'
    + p = ( (copy x: x + p x) y: y + p y ).
    - p = ( (copy x: x - p x) y: y - p y ).
    * k = ( (copy x: x * k) y: y * k ).
    r = ( (x squared + y squared) sqrt ).
    dist: p = ( ((x - p x) squared + (y - p y) squared) sqrt ) }.
  { 'Category: comparing'
    = p = ( (p _Eq: self) || [ (x = p x) && [ y = p y ] ] ) }.
  printString = ( x printString, '@', y printString )
|).

traits mirror _AddSlots: (|
  {} = 'A mirror lets you look at an object\'s slots as data.'.
  names = ( reflectee _SlotNames ).
  at: n = ( reflectee _SlotValue: n ).
  size = ( names size ).
  kind = ( reflectee _KindName ).
  addSlots: o = ( reflectee _AddSlots: o. self ).
  removeSlot: n = ( reflectee _RemoveSlot: n. self ).
  printString = ( 'a mirror on ', reflectee printString )
|).

globals _AddSlots: (|
  transcript = (|
    {} = 'The transcript window. transcript show: 42; printLine also writes here.'.
    parent* = traits oddball.
    show: s = ( s asString _StringPrint. self ).
    cr = ( '' _StringPrintLine. self ).
    clear = ( _TranscriptClear ) |).
  time = (|
    {} = 'Milliseconds since Self World started.'.
    parent* = traits oddball.
    msec = ( _TimeMsec ).
    printString = ( 'time ', _TimeMsec printString, ' ms' ) |).
  random = (|
    {} = 'random integer: 6 gives 0 to 5.'.
    parent* = traits oddball.
    integer: n = ( _Random: n ).
    between: a And: b = ( a + (_Random: (b - a) + 1) ) |).
  memory = (|
    parent* = traits oddball.
    objectCount = ( _ObjectCount ) |)
|).

lobby _AddSlots: (|
  { 'Category: examples'
  bankAccount = (|
    {} = 'The classic Self example: copy me, then deposit: and withdraw:'.
    parent* = traits clonable.
    balance <- 0.
    deposit: x = ( balance: balance + x ).
    withdraw: x = ( balance: (balance - x) max: 0 ).
    printString = ( 'an account with ', balance printString ) |).
  stack = (|
    {} = 'A stack built by inheriting from list and renaming its messages.'.
    parent* = list copyRemoveAll.
    push: x = ( add: x ).
    pop = ( removeLast ).
    top = ( last ) |) }
|).

"---------------------------------------------------------------- loading original Self files

 Original Self source files describe the world with statements such as
   bootstrap addSlotsTo: bootstrap stub -> 'globals' -> 'true' -> () From: ( | ... | )
 These objects let such files be loaded: a stub path names an object by the
 slots leading to it from the lobby, creating empty objects that are missing."

traits _AddSlots: (|
  bootstrapPath = (| parent* = traits clonable |)
|).

traits bootstrapPath _AddSlots: (|
  {} = 'A path of slot names from the lobby, built with ->; ending it with () finds the object.'.
  -> n = ( _BootstrapArrow: n ).
  printString = ( 'a bootstrap path' )
|).

globals _AddSlots: (|
  bootstrap = (|
    {} = 'Understands the statements of original Self source files.'.
    parent* = traits oddball.
    stub = ( _BootstrapStub ).
    addSlotsTo: o From: lit = ( o _AddSlots: lit. o ).
    addSlotsIfAbsentTo: o From: lit = ( o _AddSlotsIfAbsent: lit. o ).
    define: o As: lit = ( o _Define: lit. o ).
    define: o ToBe: lit = ( o _Define: lit. o ).
    setObjectAnnotationOf: o From: annotation = ( o ).
    remove: name From: o = ( o _RemoveSlot: name IfFail: [ o ]. o ).
    read: name From: dir = ( _LoadSubpart: name From: dir )
  |)
|).

traits _AddSlots: (|
  module = (|
    {} = 'Shared behaviour of module objects created by original Self source files.'.
    parent* = traits clonable.
    postFileIn = ( self ).
    printString = ( 'a module' ) |)
|).

globals _AddSlots: (|
  modules = (|
    {} = 'One object per loaded original Self source file.'.
    parent* = traits oddball |)
|).

modules _AddSlots: (|
  init = (| parent* = traits module.
            comment <- ''. directory <- ''. fileInTimeString <- ''. myComment <- ''.
            revision <- ''. subpartNames <- '' |)
|).
