# Oyzu brand guide

**Oyzu is an opinionated, batteries-included developer platform.**

Use that sentence as the primary product description. Follow it with the practical outcome: one consistent workflow for tools, environments, tasks, builds, and evidence, guided by ecosystem conventions and small, explicit exceptions. The [platform vision](../vision/VIS-001-platform.md) owns the product direction; the [implementation status](../implementation-status.md) owns capability claims.

## Logos

![Oyzu logo on light surfaces](assets/oyzu-full-logo-color.svg)

| Use | Asset |
| --- | --- |
| Full logo on light surfaces | [Color SVG](assets/oyzu-full-logo-color.svg) |
| Full logo on dark surfaces | [Reverse SVG](assets/oyzu-full-logo-reverse.svg) |
| Single-color reproduction | [Black SVG](assets/oyzu-full-logo-black.svg), [white SVG](assets/oyzu-full-logo-white.svg) |
| Compact identity or avatar | [Color mark](assets/oyzu-mark-color.svg), [reverse mark](assets/oyzu-mark-reverse.svg), [black mark](assets/oyzu-mark-black.svg), [white mark](assets/oyzu-mark-white.svg) |
| Software requiring raster images | [Color PNG](assets/oyzu-full-logo-color-960w.png), [reverse PNG](assets/oyzu-full-logo-reverse-960w.png) |
| Browser icon | [SVG favicon](assets/favicon.svg), [ICO favicon](assets/favicon.ico) |
| App and touch icons | [180 px touch icon](assets/apple-touch-icon.png), [192 px icon](assets/android-chrome-192x192.png), [512 px icon](assets/android-chrome-512x512.png) |

Use the full logo at 160 CSS pixels wide or larger; use the mark at small sizes. Preserve the aspect ratio and transparent cutouts. Allow additional clear space of about one quarter of the mark diameter. Do not stretch, rotate, add effects, or rearrange the symbol and lettering. Reverse artwork needs a dark background.

The repository README uses a light/dark `picture` element with a color-image fallback and descriptive alt text. New presentation surfaces should reuse these assets instead of redrawing the logo. Icons are available for future surfaces; adding them here does not implement a website, desktop app, or installable PWA.

## Color

| Token | Hex | Role |
| --- | --- | --- |
| Navy | `#06144D` | Wordmark on light backgrounds |
| Deep blue | `#0A1DBB` | Mark color |
| Bright blue | `#0879FF` | Mark color and brand accent |
| White | `#FFFFFF` | Reverse wordmark and light surfaces |
| Black | `#000000` | Single-color artwork |

The supplied [CSS tokens](assets/brand-colors.css) define the three brand colors. For interfaces, verify text and control contrast for the actual foreground/background pair; a logo color is not automatically a suitable text color.

## Voice and claims

Lead with the developer outcome, then explain the defaults that make it possible. Keep language direct and specific. Use **Oyzu** in prose and `oyzu` for the command. Explain “opinionated” through documented conventions and “batteries included” through integrated responsibilities.

Describe future capabilities as intended behavior. Put current limitations near getting-started instructions, including the requirement for provisioned native tools. Keep standalone use, optional managed capabilities, and the optional desktop clear. Preserve the project's AI-built, community-directed contribution model.

## Asset provenance

These assets were copied unchanged from the maintainer-supplied `oyzu-brand-kit-v1.zip`, whose notes date the pack to September 2, 2026. The SVG masters reconstruct the approved raster reference as outlined vector paths; they need no external fonts. This repository keeps a compact selection for documentation and future interfaces, rather than every print and raster export.

The [original pack notes](asset-pack-notes.txt) are retained verbatim; paths in those notes refer to the source archive, not this subset. No asset generator or third-party implementation code was imported. These files introduce no new license grant; the repository's [license status](../../README.md#license-status) remains unchanged.
