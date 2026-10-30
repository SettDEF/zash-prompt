#!/usr/bin/env bash
# Builds zash-prompt, installs it to ~/.local/bin and its zsh hook to ~/.local/share/zash-prompt,
# and sources the hook from ~/.zshrc (once). Re-run to update.
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --quiet ${JOBS:+-j "$JOBS"}
mkdir -p "$HOME/.local/bin" "$HOME/.local/share/zash-prompt"
install -m755 "${CARGO_TARGET_DIR:-target}/release/zash-prompt" "$HOME/.local/bin/zash-prompt"
install -m644 zash-prompt.zsh "$HOME/.local/share/zash-prompt/zash-prompt.zsh"
line='[[ -f ~/.local/share/zash-prompt/zash-prompt.zsh ]] && source ~/.local/share/zash-prompt/zash-prompt.zsh'
# ii's own zsh config sources utils/*.zsh itself; otherwise the line goes into ~/.zshrc
if [ -d "$HOME/.config/zsh/utils" ]; then
    ln -sf "$HOME/.local/share/zash-prompt/zash-prompt.zsh" "$HOME/.config/zsh/utils/zz-zash-prompt.zsh"
elif ! grep -qF "$line" "$HOME/.zshrc" 2>/dev/null; then
    printf '\n# the prompt (zash-prompt)\n%s\n' "$line" >> "$HOME/.zshrc"
fi
echo "zash-prompt installed: open a new zsh"
