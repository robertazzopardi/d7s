# c8s README demo — fake demo-web/demo-cache containers.
# Prereq: just c8s-demo-prep (starts them), just c8s-demo-teardown after.

Output demo.gif

Set Shell "bash"
Set Width 1400
Set Height 900
Set Theme "Catppuccin Mocha"
Set FontSize 16
Set Padding 24
Set TypingSpeed 35ms
Set Framerate 24

Hide
Type "PS1='c8s $ '"
Enter
Type "clear"
Enter
Type "./demo/run.sh"
Enter
Sleep 2s
Show

# container list → demo-web, demo-cache
Sleep 1.5s
Down@250ms 1
Sleep 1s

# restart selected container
Type "r"
Sleep 2s

# live log tail
Type "l"
Sleep 3s
Escape
Sleep 800ms

Type "q"
Sleep 800ms
