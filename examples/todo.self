"A to-do list, written in Self.

 Run it as an app:      self-world --app examples/todo.self
 or in the desktop:     World > Run a Self file as an app...

 Everything on screen is a morph: an ordinary Self object whose slots the
 desktop reads every frame. On the desktop the tasks are kept in todo.txt
 next to this file (in the browser they last until the page is closed)."

lobby _AddSlots: (|
  todoApp = (|
    {} = 'A to-do list made of morphs. Send me open.'.
    parent* = traits clonable.
    tasks <- list copyRemoveAll.
    done <- set copyRemoveAll.
    window <- nil.
    field <- nil.
    listView <- nil.
    filter <- nil.
    progress <- nil.
    status <- nil.
    file <- nil.

    { 'Category: building'
      open = ( | row. buttons. title |
        world title: 'To-do'.
        "keep the tasks next to this file, if it was loaded from one"
        bootstrap sourceFolder isNil ifFalse: [ file: bootstrap sourceFolder, '/todo.txt' ].
        window: windowMorph copyTitle: 'To-do'.
        window extent: 390 @ 470.
        window position: 60 @ 40.

        title: labelMorph copyText: 'Things to do'.
        title fontSize: 22.
        window addMorph: title.

        field: textFieldMorph copyContents: ''.
        field width: 250.
        row: rowMorph copy.
        row addMorph: field.
        row addMorph: (buttonMorph copyLabel: 'Add' Action: [ add ]).
        window addMorph: row.

        filter: dropDownMorph copyChoices: ('All' & 'Open' & 'Finished') asVector.
        filter onChange: [ refresh ].
        window addMorph: filter.

        listView: listMorph copyItems: (vector copySize: 0).
        listView extent: 350 @ 220.
        window addMorph: listView.

        buttons: rowMorph copy.
        buttons addMorph: (buttonMorph copyLabel: 'Done / not done' Action: [ toggle ]).
        buttons addMorph: (buttonMorph copyLabel: 'Remove' Action: [ remove ]).
        buttons addMorph: (buttonMorph copyLabel: 'Clear finished' Action: [ clearFinished ]).
        window addMorph: buttons.

        progress: progressMorph copyMax: 1.
        progress width: 350.
        window addMorph: progress.

        status: labelMorph copyText: ''.
        status fontSize: 13.
        status color: 'faint'.
        window addMorph: status.

        load.
        refresh.
        window openInWorld ) }.

    { 'Category: tasks'
      visible = (
        filter selected = 'Open' ifTrue: [ ^ tasks asVector reject: [| :t | done includes: t ] ].
        filter selected = 'Finished' ifTrue: [ ^ tasks asVector select: [| :t | done includes: t ] ].
        tasks asVector ).
      selectedTask = ( | i |
        i: listView selectedIndex.
        ((i < 0) || [ i >= visible size ]) ifTrue: [ ^ nil ].
        visible at: i ).
      add = ( | t |
        t: field contents trim.
        t isEmpty ifTrue: [ ^ self ].
        (tasks includes: t) ifFalse: [ tasks add: t ].
        field contents: ''.
        refresh ).
      toggle = ( | t |
        t: selectedTask.
        t isNil ifTrue: [ ^ self ].
        (done includes: t) ifTrue: [ done remove: t ] False: [ done add: t ].
        refresh ).
      remove = ( | t |
        t: selectedTask.
        t isNil ifTrue: [ ^ self ].
        tasks remove: t IfAbsent: [].
        done remove: t IfAbsent: [].
        refresh ).
      clearFinished = (
        done do: [| :t | tasks remove: t IfAbsent: [] ].
        done: set copyRemoveAll.
        refresh ) }.

    { 'Category: showing'
      refresh = (
        listView items: (visible collect: [| :t | ((done includes: t) ifTrue: '[x]  ' False: '[  ]  '), t ]).
        listView selectedIndex: -1.
        progress max: (tasks size max: 1).
        progress value: done size.
        status text: done size printString, ' of ', tasks size printString, ' done'.
        save ) }.

    { 'Category: saving'
      "One task per line: 'x ' in front of finished ones, '- ' in front of the rest."
      save = ( | text |
        file isNil ifTrue: [ ^ self ].
        text: (tasks asVector collect: [| :t | ((done includes: t) ifTrue: 'x ' False: '- '), t ]) joinUsing: '\n'.
        [ file setFileContentsTo: text ] onError: [ self ] ).
      load = (
        (file isNil || [ file fileExists not ]) ifTrue: [
          tasks add: 'Try Self World'.
          tasks add: 'Open an outliner on todoApp'.
          tasks add: 'Write an app in Self'.
          ^ self ].
        file asFileContents lines do: [| :l |
          l size > 2 ifTrue: [
            tasks add: (l copyFrom: 2).
            (l at: 0) = 'x' ifTrue: [ done add: (l copyFrom: 2) ] ] ] ) }
  |)
|).

todoApp open.
