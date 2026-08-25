# d7s README demo — sqlite sample, isolated HOME.
# Prereq: just demo

Output demo.gif

Require sqlite3

Set Shell "bash"
Set Width 1400
Set Height 900
Set Theme "Catppuccin Mocha"
Set FontSize 16
Set Padding 24
Set TypingSpeed 35ms
Set Framerate 24

Hide
Type "PS1='d7s $ '"
Enter
Type "clear"
Enter
Type "./demo/run.sh -c demo-sqlite"
Enter
Sleep 2s
Show

# tables → users (Ada, Grace, Alan, Katherine)
Sleep 1s
Down@250ms 1
Sleep 800ms
Enter
Sleep 2s

# live filter
Type "/ada"
Sleep 2s
Escape
Sleep 800ms

# k9s-style help overlay
Type "?"
Sleep 3s
Escape
Sleep 800ms

# jump to row 3
Type ":"
Sleep 200ms
Type "3"
Sleep 400ms
Enter
Sleep 1.5s

# SQL editor → join results
Type "e"
Sleep 3s
Type "E"
Sleep 3s

Escape
Sleep 1.2s
Type "q"
Sleep 800ms
