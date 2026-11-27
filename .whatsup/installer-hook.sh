#!/bin/sh
# Puts What's up ?'s guard (.whatsup/garde.py) in this clone's pre-push hook, once per developer: `sh .whatsup/installer-hook.sh`.
# Never replaces a hook: the block of .whatsup/pre-push.sh goes right after its shebang; run again, it replaces its own block.
set -e
cd "$(git rev-parse --show-toplevel)"
if [ -n "$(git config core.hooksPath)" ]; then
  echo "core.hooksPath vaut $(git config core.hooksPath) : place le bloc de .whatsup/pre-push.sh juste après le shebang de son pre-push." >&2
  exit 1
fi
hook="$(git rev-parse --git-path hooks)/pre-push"
[ -e "$hook" ] || printf '#!/bin/sh\n' > "$hook"
case "$(head -n 1 "$hook")" in
  '#!'*sh*) ;;
  *) echo "$hook n'est pas un script sh : place le bloc de .whatsup/pre-push.sh juste après son shebang." >&2; exit 1 ;;
esac
tmp=$(mktemp)
{ head -n 1 "$hook"; cat .whatsup/pre-push.sh; tail -n +2 "$hook" | sed '/^# >>> whats-up >>>$/,/^# <<< whats-up <<<$/d'; } > "$tmp"
cat "$tmp" > "$hook"
rm -f "$tmp"
chmod +x "$hook"
echo "Garde What's up ? posé dans $hook"
