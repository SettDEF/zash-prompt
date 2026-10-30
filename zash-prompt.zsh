# zash-prompt draws the prompt (folder, git, language, time taken, in tinct's colours). Source it from ~/.zshrc.
# Works alone, or after ii's pill prompt (it replaces that prompt's update_prompt_data hook).
[[ -x $HOME/.local/bin/zash-prompt ]] || return
typeset -g _zp_mode=insert _zp_ms= _zp_status=0 _zp_start=
_zp_preexec() { _zp_start=$EPOCHREALTIME }
_zp_render() {
  local -a out
  out=("${(@f)$($HOME/.local/bin/zash-prompt --status $_zp_status ${_zp_ms:+--ms} ${_zp_ms:+$_zp_ms} --mode $_zp_mode --jobs ${#jobstates})}")
  PROMPT="${out[1]}"$'\n'"${out[2]}"
  RPROMPT="${out[3]}"
  RPS2=""
}
# replaces the pill prompt's hook of the same name (sourced after it)
update_prompt_data() {
  _zp_status=$?
  if [[ -n $_zp_start ]]; then printf -v _zp_ms %.0f $(( (EPOCHREALTIME - _zp_start) * 1000 )); _zp_start=; else _zp_ms=; fi
  _zp_mode=insert
  _zp_render
}
# vim mode changes (zle-keymap-select calls this): the arrow turns
_refresh_rprompt() { [[ $KEYMAP == vicmd ]] && _zp_mode=normal || _zp_mode=insert; _zp_render }
add-zsh-hook -d preexec _prompt_timer_preexec 2>/dev/null
add-zsh-hook preexec _zp_preexec
# alone (no pill prompt): its own hooks
autoload -Uz add-zsh-hook
add-zsh-hook precmd update_prompt_data
