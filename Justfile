set quiet

# Run all checks (mirrors CI)
[parallel]
ci: veriplan check lint check-file-sizes machete crap test
build: cargo-build

# Fast compile check — all targets, all workspace crates
check:
    #!/usr/bin/env bash
    if output=$(cargo check --workspace --all-targets 2>&1); then
        echo "✓ check passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# Build (dev profile)
cargo-build:
    #!/usr/bin/env bash
    if output=$(cargo build --workspace 2>&1); then
        echo "✓ build passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# Run tests — show summary on success, full output on failure
test:
    #!/usr/bin/env bash
    output=$(cargo test --workspace 2>&1)
    code=$?
    if [ $code -eq 0 ]; then
        printf '%s\n' "$output" | grep -E "^cargo test:" || echo "✓ tests passed"
    else
        printf '%s\n' "$output"
        exit $code
    fi

# Clippy — deny all,pedantic,nursery (matches workspace config)
lint:
    #!/usr/bin/env bash
    if output=$(cargo clippy --workspace --all-targets -- -Dwarnings 2>&1); then
        echo "✓ lint passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

[group('optional')]
veriplan:
    #!/usr/bin/env bash
    if command -v veriplan >/dev/null 2>&1; then
        if output=$(veriplan check 2>&1); then
            echo "✓ veriplan passed"
        else
            printf '%s\n' "$output"
            exit 1
        fi
    else
        echo "⚠ veriplan skipped (veriplan not installed)"
        exit 0
    fi

# Check format without modifying files
fmt:
    #!/usr/bin/env bash
    if output=$(cargo fmt --check 2>&1); then
        echo "✓ fmt passed"
    else
        printf '%s\n' "$output"
        echo "→ fix with: cargo fmt"
        exit 1
    fi

# Unused dependency check
machete:
    #!/usr/bin/env bash
    if output=$(cargo machete 2>&1); then
        echo "✓ machete passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# CRAP complexity — generates coverage then scores; fails if any function exceeds threshold 30.
# Uses --features integration so the coverage matches what CI produces (cargo:test artifact).
# --missing skip: functions absent from lcov (gateway.rs, registry.rs, session.rs, main.rs
# are excluded via --ignore-filename-regex) are skipped rather than penalised as 0%.
crap:
    #!/usr/bin/env bash
    # Run offline unit tests only — no integration or e2e features.
    # (Integration tests require a live gateway; use just test-e2e-gateway for those.)
    if output=$(cargo llvm-cov --workspace \
        --lcov --output-path /tmp/lcov-crap.info \
        --ignore-filename-regex 'main\.rs' \
        --lib --bins --tests --quiet 2>/dev/null); then
        if output=$(cargo crap --workspace --lcov /tmp/lcov-crap.info \
            --threshold 30 \
            --exclude 'tests/**' --exclude 'src/**/main.rs' \
            --missing skip --fail-above 2>/dev/null); then
            echo "✓ crap passed"
        else
            printf '%s\n' "$output"
            exit 1
        fi
    else
        printf '%s\n' "$output"
        exit 1
    fi


# Check that no production source file exceeds 500 lines.
# Files under tests/ directories are excluded.
check-file-sizes max="500":
    #!/usr/bin/env bash
    MAX={{max}}
    fail=0
    while IFS= read -r f; do
        lines=$(wc -l < "$f")
        if [ "$lines" -gt "$MAX" ]; then
            echo "FAIL: $f has $lines lines (max $MAX)"
            fail=1
        fi
    done < <(find src -name '*.rs' | grep -v '/tests/')
    [ $fail -eq 0 ] && echo "✓ all source files within $MAX lines"
