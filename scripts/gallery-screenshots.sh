#!/usr/bin/env bash
# Render the README widget gallery to docs/images/gallery/ (WI-1910).
#
#   scripts/gallery-screenshots.sh            check the committed images; fail if any differ
#   scripts/gallery-screenshots.sh --update   rewrite the images that differ, then optimise them
#
# The images come from crates/gallery/tests/gallery_snapshots.rs through a wgpu adapter. Pixels
# depend on the rasteriser, so the script runs in one pinned image with Mesa lavapipe (a
# software Vulkan driver). The `gallery-screenshots` CI job runs this script in the same
# image, so a local --update and CI agree. Needs Docker unless already inside that image.
set -euo pipefail

IMAGE="rust:1.95-trixie"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mode="${1:-check}"
case "$mode" in
  check | --update) ;;
  *) echo "usage: $0 [--update]" >&2; exit 2 ;;
esac

if [[ "${GALLERY_SHOTS_IN_CONTAINER:-}" != 1 ]]; then
  # Stream the checkout in and the images back out as tar rather than bind-mounting, so it
  # also works against a Docker daemon that cannot see this filesystem. The container's
  # stdout carries only the tar; its log goes to stderr.
  cd "$root"
  git ls-files -z --cached --others --exclude-standard \
    | tar --null -T - -cf - \
    | docker run --rm -i \
        -e GALLERY_SHOTS_IN_CONTAINER=1 \
        -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-}" \
        -v fluent2-egui-cargo-registry:/usr/local/cargo/registry \
        -v fluent2-egui-gallery-target:/target \
        -e CARGO_TARGET_DIR=/target \
        "$IMAGE" bash -c "mkdir -p /work && tar -xf - -C /work && cd /work \
          && scripts/gallery-screenshots.sh $mode >&2; s=\$?; mkdir -p docs/images/gallery; tar -cf - docs/images/gallery; exit \$s" \
    | tar -xf - -C "$root"
  exit "${PIPESTATUS[2]}"
fi

export DEBIAN_FRONTEND=noninteractive
if ! [[ -e /usr/share/vulkan/icd.d/lvp_icd.json ]]; then
  apt-get update -qq
  apt-get install -y -qq --no-install-recommends mesa-vulkan-drivers >/dev/null
fi
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export WGPU_BACKEND=vulkan
# Keep the container's build apart from the host's target/.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/gallery-shots}"

status=0
if [[ "$mode" == --update ]]; then
  UPDATE_SNAPSHOTS=1 cargo test -p gallery --features snapshots --test gallery_snapshots || status=$?
  # Lossless, so the pixel check is unaffected; an image that is already optimal is left
  # alone. Debian has no oxipng package, so build it once into the target cache.
  export PATH="$CARGO_TARGET_DIR/tools/bin:$PATH"
  command -v oxipng >/dev/null \
    || cargo install -q --locked --root "$CARGO_TARGET_DIR/tools" oxipng --version 9.1.5
  oxipng -q -o 4 --strip safe docs/images/gallery/*.png
else
  cargo test -p gallery --features snapshots --test gallery_snapshots || status=$?
fi

exit "$status"
