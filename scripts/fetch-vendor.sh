#!/bin/sh
# 下载前端第三方库到 assets/vendor（版本固定，离线打包进 exe）
set -e
cd "$(dirname "$0")/../assets"
CDN=https://cdn.jsdelivr.net/npm
get() { mkdir -p "vendor/$(dirname "$2")"; curl -fsSL "$CDN/$1" -o "vendor/$2"; }

get markdown-it@14.1.0/dist/markdown-it.min.js               markdown-it.min.js
get markdown-it-task-lists@2.1.1/dist/markdown-it-task-lists.min.js markdown-it-task-lists.min.js
get markdown-it-footnote@4.0.0/dist/markdown-it-footnote.min.js markdown-it-footnote.min.js
get markdown-it-texmath@1.0.0/texmath.js                      texmath.js
get katex@0.16.22/dist/katex.min.js                           katex/katex.min.js
get katex@0.16.22/dist/katex.min.css                          katex/katex.min.css
for f in $(grep -o 'fonts/[A-Za-z0-9_-]*\.woff2' vendor/katex/katex.min.css | sort -u); do
  get "katex@0.16.22/dist/$f" "katex/$f"
done
get @highlightjs/cdn-assets@11.11.1/highlight.min.js           hljs/highlight.min.js
get @highlightjs/cdn-assets@11.11.1/styles/github.min.css      hljs/github.min.css
get @highlightjs/cdn-assets@11.11.1/styles/github-dark.min.css hljs/github-dark.min.css
get codemirror@5.65.19/lib/codemirror.js                       cm/codemirror.js
get codemirror@5.65.19/lib/codemirror.css                      cm/codemirror.css
get codemirror@5.65.19/addon/mode/overlay.js                   cm/overlay.js
get codemirror@5.65.19/mode/markdown/markdown.js               cm/markdown.js
get codemirror@5.65.19/mode/gfm/gfm.js                         cm/gfm.js
