<!-- Thanks for opening a Pull Request! Please fill in the sections below. -->

## Summary

<!-- What does this PR do and why? -->

## Related issue

<!-- Replace the placeholder below with the issue number this PR resolves. -->

Closes #N

## Type of change

<!-- Put an `x` in the boxes that apply. -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation only
- [ ] Tooling / CI only

## Test evidence

<!-- Show that this change is tested. Paste the commands you ran and their output. -->

```
cargo fmt --all -- -inheader
cargo clippy --workspace --all-targets --all-features - -D warnings
cargo test --workspace
```

## Checklist

- [ ] My code follows the style guidelines of this project.
- [ ] My changes require a documentation update, and I have updated the docs accordingly.
- [ ] All new and existing tests pass locally with my changes.
- [ ] Any dependent changes have been merged and published in dependent repositories.

## Additional notes

<!-- Anything else the reviewer should know. -->
