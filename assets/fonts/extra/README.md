# Extra fonts

Put `.ttf` or `.otf` files here to embed them in the Folio you build, for
example Anthropic Sans:

```
assets/fonts/extra/AnthropicSans-Regular.ttf
assets/fonts/extra/AnthropicSans-Medium.ttf
assets/fonts/extra/AnthropicSans-Semibold.ttf
assets/fonts/extra/AnthropicSans-Bold.ttf
```

then build as usual. When the font named Anthropic Sans is in the app, the
whole interface uses it (`src/fonts.rs`).

Only put fonts here that you are licensed to redistribute inside an
application. Everything in this folder except this file is ignored by git,
so the font files never reach the repository.
