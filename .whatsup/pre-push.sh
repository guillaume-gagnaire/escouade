# >>> whats-up >>>
# Inserted right after the shebang of a pre-push hook: reads git's stdin once, then hands it back to the rest of the hook.
if [ -f .whatsup/garde.py ] && command -v python3 >/dev/null 2>&1; then
  wu_in=$(cat)
  printf '%s\n' "$wu_in" | python3 .whatsup/garde.py pre-push "$@"
  [ $? -eq 3 ] && exit 1
  exec 0<<WHATSUP_FIN
$wu_in
WHATSUP_FIN
fi
# <<< whats-up <<<
