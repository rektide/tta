#!/usr/bin/env zsh
# Real completion widgets in an isolated PTY; no launch command is submitted.
emulate -LR zsh
setopt err_exit no_unset pipe_fail
zmodload zsh/zpty
zmodload zsh/zselect

binary=$1
export TTA_COMPLETION_FUNCTIONS=$2
fixture=$3
export TTA_LIVE_SOCKET=$4
mkdir -p "$fixture/bin" "$fixture/root/pipewire-native" "$fixture/root/space project" \
  "$fixture/root/meta[x]*" "$fixture/root/quote'project" "$fixture/root/--dash" \
  "$fixture/override root/other-project"
print -r -- 'not a socket' > "$fixture/bad-socket"
ln -s "$binary" "$fixture/bin/tta"
export PATH="$fixture/bin:$PATH" HOME="$fixture"
for key in ${(k)parameters[(I)TTA_*]}; do
  [[ $key == (TTA_COMPLETION_FUNCTIONS|TTA_LIVE_SOCKET) ]] || unset "$key"
done
unset PROJECTS_DIR TMUX TMUX_PANE
export TTA_ROOT="$fixture/root" TTA_SOCKET_PATH="$fixture/socket"
export TTA_CAPTURE="$fixture/capture"

zpty -b completion env TERM=xterm zsh -f
trap 'zpty -d completion' EXIT
zpty -w completion 'PS1="READY> "; fpath=( "$TTA_COMPLETION_FUNCTIONS" $fpath ); autoload -Uz compinit; compinit -D; bindkey "^I" complete-word; _capture() { print -r -- "$BUFFER" > "$TTA_CAPTURE"; BUFFER=""; zle .accept-line }; zle -N _capture; bindkey "^X" _capture; print INITIALIZED'

transcript=''
wait_for() {
  local needle=$1 chunk i
  for i in {1..1000}; do
    if zpty -r completion chunk; then
      transcript+=$chunk
      [[ $transcript == *$needle* ]] && return 0
    fi
    zselect -t 1 || true
  done
  print -u2 -r -- "Timed out waiting for $needle: $transcript"
  return 1
}
wait_for INITIALIZED

check() {
  local input=$1 expected=$2 i actual
  rm -f "$TTA_CAPTURE"
  zpty -w -n completion "$input"$'\t\x18'
  for i in {1..1000}; do
    if [[ -f $TTA_CAPTURE ]]; then
      actual=$(<"$TTA_CAPTURE")
      if [[ $actual != "$expected" ]]; then
        print -u2 -r -- "Input ${(qqq)input}: expected ${(qqq)expected}, got ${(qqq)actual}"
        return 1
      fi
      local chunk
      while zpty -r completion chunk; do :; done
      return 0
    fi
    zselect -t 1 || true
  done
  print -u2 -r -- "Timed out completing ${(qqq)input}"
  return 1
}

check 'tta pipe' 'tta pipewire-native '
check 'tta wire' 'tta pipewire-native '
check 'tta *wire*nat*' 'tta pipewire-native '
check 'tta p?pewire-nat*' 'tta pipewire-native '
check 'tta space' 'tta space\ project '
check 'tta meta' 'tta meta\[x\]\* '
check 'tta quote' "tta quote\\'project "
check 'tta --roo' 'tta --root='
check "tta --root '$fixture/override root' oth" "tta --root '$fixture/override root' other-project "
check "tta --root='$fixture/override root' oth" "tta --root='$fixture/override root' other-project "
check "tta --socket-path '$TTA_LIVE_SOCKET' *work" "tta --socket-path '$TTA_LIVE_SOCKET' live-work "
check "tta --socket-path='$TTA_LIVE_SOCKET' live" "tta --socket-path='$TTA_LIVE_SOCKET' live-work "
check "tta --root /no-such-root --socket-path '$TTA_LIVE_SOCKET' work" "tta --root /no-such-root --socket-path '$TTA_LIVE_SOCKET' live-work "
check "tta --socket-path '$fixture/bad-socket' wire" "tta --socket-path '$fixture/bad-socket' pipewire-native "
check 'tta --session-name renamed wire' 'tta --session-name renamed pipewire-native '
check 'tta -- --da' 'tta -- --dash '
check 'tta -- --not-a-project' 'tta -- --not-a-project'
[[ ! -e $TTA_SOCKET_PATH ]]
