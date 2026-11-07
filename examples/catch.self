"Catch the stars: a little game written in Self.

 Run it as an app:      self-world --app examples/catch.self
 or in the desktop:     World > Run a Self file as an app...

 The arrow keys (or A and D) move the paddle; Space starts again.
 The game is a plain object; the score label is a morph that wants keys
 (wantsKeys) and steps 30 times a second, passing both on to the game."

lobby _AddSlots: (|
  catchGame = (|
    {} = 'Catch the falling stars with the paddle. Send me start.'.
    parent* = traits clonable.
    paddle <- nil.
    scoreLabel <- nil.
    stars <- list copyRemoveAll.
    score <- 0.
    missed <- 0.
    dx <- 0.
    ticks <- 0.
    playing <- false.

    { 'Category: starting'
      start = (
        world title: 'Catch the stars'.
        world clear.
        paddle: rectangleMorph copy.
        paddle _AddSlots: (| cornerRadius = 8 |).
        paddle extent: 130 @ 16.
        paddle color: 'teal'.
        paddle openInWorld.

        scoreLabel: labelMorph copyText: ''.
        scoreLabel fontSize: 22.
        scoreLabel position: 20 @ 14.
        scoreLabel _AddSlots: (| game <- nil.
          step = ( game step ).
          keyDown: k = ( game keyDown: k ).
          keyUp: k = ( game keyUp: k ) |).
        scoreLabel game: self.
        scoreLabel wantsKeys: true.
        scoreLabel openInWorld.
        scoreLabel startStepping.
        restart ).
      restart = (
        stars do: [| :s | s delete ].
        stars: list copyRemoveAll.
        score: 0. missed: 0. ticks: 0. dx: 0.
        paddle position: ((world width - paddle width) / 2) @ (world height - 60).
        playing: true ) }.

    { 'Category: playing'
      keyDown: k = (
        ((k = 'ArrowLeft') || [ k = 'A' ]) ifTrue: [ dx: -14 ].
        ((k = 'ArrowRight') || [ k = 'D' ]) ifTrue: [ dx: 14 ].
        k = 'Space' ifTrue: [ restart ] ).
      keyUp: k = ( dx: 0 ).
      step = (
        playing ifFalse: [ ^ self ].
        paddle x: ((paddle x + dx) max: 0) min: world width - paddle width.
        paddle y: world height - 60.
        ticks: ticks + 1.
        (ticks % 24) = 0 ifTrue: [ drop ].
        stars copy do: [| :s |
          s y: s y + s speed.
          (caught: s)
            ifTrue: [ score: score + 1. forget: s ]
            False: [ s y > world height ifTrue: [ missed: missed + 1. forget: s ] ] ].
        missed >= 10 ifTrue: [ playing: false ].
        scoreLabel text: (playing
          ifTrue: [ 'Score: ', score printString, '    Missed: ', missed printString, ' of 10' ]
          False: [ 'Game over! Score: ', score printString, '. Press Space to play again.' ]) ).
      drop = ( | s |
        s: circleMorph copy.
        s _AddSlots: (| speed <- 4 |).
        s speed: 3 + (random integer: 5).
        s extent: 26 @ 26.
        s color: (('gold' & 'tomato' & 'pink' & 'cyan') asVector at: (random integer: 4)).
        s position: (random integer: ((world width - 40) max: 1)) @ 50.
        stars add: s.
        s openInWorld ).
      caught: s = (
        ((s y + s height) >= paddle y)
          && [ (s y <= (paddle y + paddle height))
          && [ ((s x + s width) >= paddle x)
          && [ s x <= (paddle x + paddle width) ] ] ] ).
      forget: s = ( stars remove: s IfAbsent: []. s delete ) }
  |)
|).

catchGame start.
