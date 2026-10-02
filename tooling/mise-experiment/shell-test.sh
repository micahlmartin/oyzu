# Executed explicitly by Bash or Zsh; uses the upstream-generated activation hook.
set -e
export OYZU_PROJECT=before
export PATH="$OYZU_TEST_EXTRA:$OYZU_TEST_EXTRA:$PATH"
initial_path=$PATH
eval "$("$OYZU_SPIKE_BINARY" activate -s "$OYZU_TEST_SHELL")"
cd "$OYZU_TEST_A"
_mise_hook
test "$(node --version)" = "$OYZU_EXPECT_A"
test "$OYZU_PROJECT" = "$OYZU_EXPECT_ENV_A"
cd "$OYZU_TEST_B"
_mise_hook
test "$(node --version)" = "$OYZU_EXPECT_B"
test "$OYZU_PROJECT" = "$OYZU_EXPECT_ENV_B"
cd "$OYZU_TEST_OUTSIDE"
_mise_hook
test "$OYZU_PROJECT" = before
test "$PATH" = "$initial_path"
cd "$OYZU_TEST_A"
_mise_hook
export OYZU_PROJECT=user-change
export PATH="$OYZU_TEST_EXTRA:$PATH"
cd "$OYZU_TEST_OUTSIDE"
_mise_hook
test "$OYZU_PROJECT" = user-change
test "$PATH" = "$OYZU_TEST_EXTRA:$initial_path"
printf 'shell lifecycle passed\n'
