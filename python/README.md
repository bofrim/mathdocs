# mathrender

`mathrender` provides tiny metadata helpers used by the editor-side MathRender
renderer. The helpers make Python source valid and type-checkable; rendering
tools read the source statically and do not import target modules.

Top-level `render_figure(...)` calls are static placement directives. They tell
the renderer where to include an already generated plot, diagram, screenshot, or
other image in the final Markdown document. `render_image(...)` and
`render_plot(...)` remain available as aliases.
