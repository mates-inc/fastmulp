#!/bin/sh
set -eu

usage() {
  echo "usage: ./scripts/release.sh <patch|minor|alpha|beta>" >&2
}

ensure_git_repo() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
    echo "release workflow requires a git repository" >&2
    exit 1
  }
}

ensure_clean_worktree() {
  git diff --quiet --ignore-submodules HEAD -- || {
    echo "release workflow requires a clean worktree" >&2
    exit 1
  }
}

read_workspace_version() {
  awk '
    /^\[workspace\.package\]/ { in_section=1; next }
    /^\[/ && in_section { exit }
    in_section && /^version = "/ {
      gsub(/^version = "/, "", $0)
      gsub(/"$/, "", $0)
      print
      exit
    }
  ' Cargo.toml
}

write_workspace_version() {
  new_version="$1"
  awk -v version="$new_version" '
    BEGIN { in_section=0; updated=0 }
    /^\[workspace\.package\]/ { in_section=1; print; next }
    /^\[/ && in_section { in_section=0 }
    in_section && !updated && /^version = "/ {
      print "version = \"" version "\""
      updated=1
      next
    }
    { print }
  ' Cargo.toml > Cargo.toml.tmp
  mv Cargo.toml.tmp Cargo.toml
}

refresh_lockfile() {
  cargo metadata --format-version 1 >/dev/null
}

ensure_locked_metadata() {
  cargo metadata --locked --format-version 1 >/dev/null
}

bump_patch_base() {
  major="$1"
  minor="$2"
  patch="$3"
  echo "${major}.${minor}.$((patch + 1))"
}

compute_next_version() {
  current="$1"
  kind="$2"
  base="${current%%-*}"
  prerelease=""
  if [ "$base" != "$current" ]; then
    prerelease="${current#"$base"-}"
  fi

  old_ifs="${IFS}"
  IFS=.
  set -- $base
  IFS="${old_ifs}"
  major="$1"
  minor="$2"
  patch="$3"

  prerelease_name=""
  prerelease_number=""
  if [ -n "$prerelease" ]; then
    prerelease_name="${prerelease%%.*}"
    prerelease_number="${prerelease#"$prerelease_name".}"
  fi

  case "$kind" in
    patch)
      echo "$(bump_patch_base "$major" "$minor" "$patch")"
      ;;
    minor)
      echo "${major}.$((minor + 1)).0"
      ;;
    alpha)
      if [ "$prerelease_name" = "alpha" ]; then
        echo "${base}-alpha.$((prerelease_number + 1))"
        return
      fi
      echo "$(bump_patch_base "$major" "$minor" "$patch")-alpha.0"
      ;;
    beta)
      if [ "$prerelease_name" = "beta" ]; then
        echo "${base}-beta.$((prerelease_number + 1))"
        return
      fi
      if [ "$prerelease_name" = "alpha" ]; then
        echo "${base}-beta.0"
        return
      fi
      echo "$(bump_patch_base "$major" "$minor" "$patch")-beta.0"
      ;;
    *)
      usage
      exit 1
      ;;
  esac
}

if [ $# -ne 1 ]; then
  usage
  exit 1
fi

kind="$1"
case "$kind" in
  patch|minor|alpha|beta) ;;
  *)
    usage
    exit 1
    ;;
esac

ensure_git_repo
ensure_clean_worktree
ensure_locked_metadata

current_version="$(read_workspace_version)"
if [ -z "$current_version" ]; then
  echo "failed to read workspace version from Cargo.toml" >&2
  exit 1
fi

next_version="$(compute_next_version "$current_version" "$kind")"
if git rev-parse "v${next_version}" >/dev/null 2>&1; then
  echo "tag v${next_version} already exists" >&2
  exit 1
fi

write_workspace_version "$next_version"
refresh_lockfile
ensure_locked_metadata
git add Cargo.toml Cargo.lock
git commit -m "release: v${next_version}"
git tag "v${next_version}"

echo "created release commit and tag for v${next_version}"
