# Operator reference

Generated from the operator registry — the same table the editor builds its menus and parameter pages from, so this cannot drift from what the operators actually do.

186 operators.

**CHOP** (53)

- [analyzeCHOP](#analyze--analyzechop) — Reduce each channel to one number.
- [animationCHOP](#animation--animationchop) — Keyframed curves over time.
- [audiodeviceinCHOP](#audio-device-in--audiodeviceinchop) — Samples from an audio input, downmixed to mono.
- [audiodeviceoutCHOP](#audio-device-out--audiodeviceoutchop) — Plays its first channel on an audio output.
- [audiofileinCHOP](#audio-file-in--audiofileinchop) — Plays an audio file, following the timeline.
- [audiospectrumCHOP](#audio-spectrum--audiospectrumchop) — Energy per frequency band, log spaced.
- [beatCHOP](#beat--beatchop) — A tempo clock: ramp, pulse, count and bar.
- [blobtrackCHOP](#blob-track--blobtrackchop) — Largest connected bright region. Channels: present, count, x, y, area, width, height, vx, vy. Coordinates and sizes are 0..1, origin top-left; velocity is frame-fractions/second. CPU readback of last frame; downsample to 160x90 first. Not person recognition or persistent identity: switching largest regions can jump. A missing/empty input returns zero channels values.
- [clockCHOP](#clock--clockchop) — Wall-clock time, which keeps running when the timeline is paused.
- [constantCHOP](#constant--constantchop) — Fixed values as channels.
- [countCHOP](#count--countchop) — Count threshold crossings.
- [crossCHOP](#cross--crosschop) — Blend two inputs, with an equal-power option.
- [dattochopCHOP](#dat-to-chop--dattochopchop) — Columns or rows of a table, as channels.
- [delayCHOP](#delay--delaychop) — Play a channel back later, with optional feedback.
- [dmxoutCHOP](#dmx-out--dmxoutchop) — Sends its channels as DMX over Art-Net or sACN.
- [expressionCHOP](#expression--expressionchop) — Apply an expression to every sample. `v` is the sample.
- [fanCHOP](#fan--fanchop) — Fan out an index channel to one on/off channel per value (chan0..chanN-1), or fan many channels in to the index of the first one above zero (-1 if none).
- [filterCHOP](#filter--filterchop) — Low-pass or moving-average smoothing.
- [holdCHOP](#hold--holdchop) — Freeze input 1's value until input 2 fires.
- [inCHOP](#in--inchop) — A channel input on this component's node.
- [infoCHOP](#info--infochop) — Another operator's info channels (TouchDesigner Info CHOP).
- [keyboardinCHOP](#keyboard-in--keyboardinchop) — One channel per named key, 1 while held.
- [lagCHOP](#lag--lagchop) — Smooth a channel, with separate rise and fall times.
- [lfoCHOP](#lfo--lfochop) — A repeating waveform over time.
- [limitCHOP](#limit--limitchop) — Clamp, wrap, fold or quantise a channel.
- [logicCHOP](#logic--logicchop) — Turn channels into on/off and combine them.
- [mathCHOP](#math--mathchop) — Combine and scale channels.
- [mergeCHOP](#merge--mergechop) — Put two CHOPs' channels side by side.
- [midiinCHOP](#midi-in--midiinchop) — Notes, velocity, pitch bend and the controls you have moved.
- [midioutCHOP](#midi-out--midioutchop) — Sends `nNN` channels as notes and `ccNN` channels as controls.
- [mouseinCHOP](#mouse-in--mouseinchop) — Cursor position and buttons.
- [noiseCHOP](#noise--noisechop) — Sparse convolution noise over time; channel count comes from Channel Names.
- [nullCHOP](#null--nullchop) — Pass-through. A stable name to export from.
- [oscinCHOP](#osc-in--oscinchop) — Incoming OSC messages, one channel per address argument.
- [oscoutCHOP](#osc-out--oscoutchop) — Sends each channel as an OSC message to the network address and port.
- [outCHOP](#out--outchop) — This component's channel output.
- [panelCHOP](#panel--panelchop) — Panel widget values as channels.
- [parameterCHOP](#parameter--parameterchop) — Another operator's parameter values as channels (does not cook that operator).
- [particleCHOP](#particle--particlechop) — Stateful 2D particle emitter with velocity, gravity, lifetime and respawn. tx/ty/tz are instance positions (6 scene units across); size/life fade to zero. Emitter coordinates are normalized, origin top-left. Connect via geometryCOMP.instancechop; set sx/sy/sz to size. Active stops births; existing particles finish. Reset clears the simulation. CPU, at most 4096 particles; dt capped at 0.1s; no collisions or flocking.
- [patternCHOP](#pattern--patternchop) — A fixed-length waveform buffer.
- [renameCHOP](#rename--renamechop) — Rename channels by pattern.
- [resampleCHOP](#resample--resamplechop) — Change how many samples a buffer has, keeping its shape.
- [selectCHOP](#select--selectchop) — Pick and rename channels.
- [shuffleCHOP](#shuffle--shufflechop) — Rearrange the channel/sample grid — transpose, reverse, sequence.
- [slopeCHOP](#slope--slopechop) — The rate of change of a channel.
- [soptochopCHOP](#sop-to-chop--soptochopchop) — Point attributes as channels, one sample per point.
- [speedCHOP](#speed--speedchop) — Integrate a channel: velocity becomes position.
- [springCHOP](#spring--springchop) — Follow a channel like a mass on a spring: overshoot, wobble, then settle. Raise damping to stop the wobble; raise the spring constant to follow faster.
- [switchCHOP](#switch--switchchop) — Choose one of two inputs.
- [timerCHOP](#timer--timerchop) — A running timer with fraction, seconds, cycles and done.
- [toptochopCHOP](#top-to-chop--toptochopchop) — Pixels as channels r g b a. `image` reads every pixel (top row first, at most 128x128) and adds u/v positions, centred, v up — instance a Geometry COMP with tx=u ty=v tz=g for a camera point cloud. Reads last frame; costs a GPU sync, so put a Resolution TOP in front.
- [trailCHOP](#trail--trailchop) — The last few seconds of each channel as one buffer, oldest sample first — draw a waveform, or lay out one instance per sample.
- [triggerCHOP](#trigger--triggerchop) — An attack/decay/sustain/release envelope per channel.

**COMP** (18)

- [baseCOMP](#base--basecomp) — Sub-network; In and Out operators inside define inputs and outputs.
- [buttonCOMP](#button--buttoncomp) — Toggle or momentary control; interior textCOMP + textTOP draws the label.
- [cameraCOMP](#camera--cameracomp) — A viewpoint for a Render TOP.
- [containerCOMP](#container--containercomp) — Parent panel for buttons, sliders, text, OP viewers, and nested containers; Children page drives align/layout.
- [fieldCOMP](#field--fieldcomp) — Editable string parameter with textCOMP/textTOP interior.
- [geometryCOMP](#geometry--geometrycomp) — Places a SOP in the scene, optionally instanced.
- [glslCOMP](#glsl--glslcomp) — Panel region filled from an interior glslTOP network (layout only until cook hook lands).
- [lightCOMP](#light--lightcomp) — A directional light, aimed from its position at the origin.
- [listCOMP](#list--listcomp) — Scrollable list panel — parameters match TD; rendering uses interior TOPs.
- [opviewerCOMP](#op-viewer--opviewercomp) — Shows another operator's cooked TOP inside this panel rect.
- [parameterCOMP](#parameter--parametercomp) — Auto-layout of another operator's custom parameters (stub — build from par + TOP).
- [replicatorCOMP](#replicator--replicatorcomp) — Keeps one clone of a master component per row of a table.
- [selectCOMP](#select--selectcomp) — Displays another panel COMP by path (stub — use opviewer + container for now).
- [sliderCOMP](#slider--slidercomp) — Fader control; Value parameter is the state the network reads.
- [tableCOMP](#table--tablecomp) — Tabular panel tied to a tableDAT (stub — interior TOP/DAT wiring).
- [textCOMP](#text--textcomp) — Static or expression-driven text via interior textTOP.
- [widgetCOMP](#widget--widgetcomp) — Reusable panel asset hook (stub — same graph-in-COMP model as other panels).
- [windowCOMP](#window--windowcomp) — Floating or external display for a cooked TOP (TouchDesigner Window COMP).

**DAT** (22)

- [chopexecuteDAT](#chop-execute--chopexecutedat) — Python callbacks when a watched channel changes or crosses a threshold.
- [choptodatDAT](#chop-to-dat--choptodatdat) — Channels as a table of numbers.
- [convertDAT](#convert--convertdat) — Between a table and a block of text.
- [executeDAT](#execute--executedat) — Python callbacks at the start and end of a frame.
- [inDAT](#in--indat) — A data input on this component's node.
- [jsonDAT](#json--jsondat) — Parse JSON text into a path/value table.
- [mergeDAT](#merge--mergedat) — Join two DATs by rows or by columns.
- [nullDAT](#null--nulldat) — Pass-through. A stable name to reference.
- [outDAT](#out--outdat) — This component's data output.
- [panelexecuteDAT](#panel-execute--panelexecutedat) — Python callbacks when a panel value changes.
- [parameterexecuteDAT](#parameter-execute--parameterexecutedat) — Python callbacks when a watched parameter changes.
- [scriptDAT](#script--scriptdat) — Rows produced by a Python script.
- [selectDAT](#select--selectdat) — Pick rows and columns, by name or index.
- [sortDAT](#sort--sortdat) — Sort rows by a column, numerically or as text.
- [substituteDAT](#substitute--substitutedat) — Fill $name placeholders from a two-column lookup table.
- [tableDAT](#table--tabledat) — A table of text, stored in the project file.
- [textDAT](#text--textdat) — A block of text.
- [transposeDAT](#transpose--transposedat) — Swap rows and columns.
- [udpinDAT](#udp-in--udpindat) — Listens for datagrams; table columns address, port, message.
- [udpoutDAT](#udp-out--udpoutdat) — Sends with send() / optional Send Input On Change; table shows incoming replies.
- [webclientDAT](#web-client--webclientdat) — HTTP client: pulse Fetch to request in a background thread; response body is the DAT contents. WebSocket is not implemented yet.
- [webserverDAT](#web-server--webserverdat) — HTTP server: onHTTPRequest and WebSocket callbacks in the Callbacks DAT; HTTP and WebSocket on the same port.

**MAT** (5)

- [constantMAT](#constant--constantmat) — Flat colour, unaffected by lights.
- [pbrMAT](#pbr--pbrmat) — Base colour, metallic, roughness and emission, with an optional map.
- [phongMAT](#phong--phongmat) — Diffuse and a Blinn highlight, dialled by shininess.
- [pointspriteMAT](#point-sprite--pointspritemat) — Draws every point as a camera-facing quad.
- [wireframeMAT](#wireframe--wireframemat) — Draws the edges rather than the faces.

**SOP** (18)

- [blendSOP](#blend--blendsop) — Morph between two shapes by interpolating point positions.
- [boxSOP](#box--boxsop) — A box with flat-shaded faces.
- [circleSOP](#circle--circlesop) — A disc, ring or arc — filled, or a line to copy along.
- [colorSOP](#color--colorsop) — Set the colour carried by every point.
- [copySOP](#copy--copysop) — Stamp copies with a compounding transform.
- [facetSOP](#facet--facetsop) — Recompute normals from the triangles — put after Noise or Twist so lighting follows the new shape. `unique` gives flat, faceted shading.
- [gridSOP](#grid--gridsop) — A flat grid of quads — the usual thing to displace.
- [inSOP](#in--insop) — A geometry input on this component's node.
- [lineSOP](#line--linesop) — A run of points between two positions.
- [mergeSOP](#merge--mergesop) — Combine two pieces of geometry.
- [noiseSOP](#noise--noisesop) — Displace points by value noise, along their normals or freely.
- [nullSOP](#null--nullsop) — Pass-through. A stable name to reference.
- [outSOP](#out--outsop) — This component's geometry output.
- [sphereSOP](#sphere--spheresop) — A UV sphere.
- [torusSOP](#torus--torussop) — A torus.
- [transformSOP](#transform--transformsop) — Translate, rotate and scale points.
- [tubeSOP](#tube--tubesop) — A cylinder, cone or tapered tube, with optional caps.
- [twistSOP](#twist--twistsop) — Twist, bend or taper along one axis, growing with distance from the origin. Needs enough rows along that axis to curve smoothly.

**TOP** (70)

- [addTOP](#add--addtop) — The two inputs summed.
- [antialiasTOP](#anti-alias--antialiastop) — Softens luminance edges in place — FXAA as a post pass.
- [blurTOP](#blur--blurtop) — Separable Gaussian blur.
- [cacheTOP](#cache--cachetop) — Holds the last frame it saw when Active is off.
- [channelmixTOP](#channel-mix--channelmixtop) — Each output channel is a weighted sum of the input's four.
- [choptotopTOP](#chop-to-top--choptotoptop) — Channels as pixels: a row per channel, a column per sample.
- [chromakeyTOP](#chroma-key--chromakeytop) — Key out one colour, matching on hue rather than brightness.
- [circleTOP](#circle--circletop) — An antialiased disc or ellipse.
- [compositeTOP](#composite--compositetop) — Blend two inputs. Input 2 is composited over input 1.
- [constantTOP](#constant--constanttop) — A flat colour at a chosen resolution.
- [convolveTOP](#convolve--convolvetop) — An arbitrary 3x3 kernel: sharpen, blur, edge detect.
- [cornerpinTOP](#corner-pin--cornerpintop) — Drag the image's four corners anywhere — a perspective warp.
- [cropTOP](#crop--croptop) — Keep a rectangle of the source, at an explicit output size.
- [crossTOP](#cross--crosstop) — Each input survives where the other is not.
- [differenceTOP](#difference--differencetop) — The absolute difference between the inputs.
- [displaceTOP](#displace--displacetop) — Offsets input 1's lookup by channels of input 2.
- [ditherTOP](#dither--dithertop) — Quantise to few levels, with an ordered or noise dither.
- [edgeTOP](#edge--edgetop) — Sobel edge detection.
- [embossTOP](#emboss--embosstop) — A directional difference, sat at mid grey.
- [feedbackTOP](#feedback--feedbacktop) — Last frame's output of the Target TOP.
- [fitTOP](#fit--fittop) — Resample into a new resolution: fill, fit, stretch or native.
- [flipTOP](#flip--fliptop) — Mirror the image about an axis, or transpose it.
- [flowTOP](#flow--flowtop) — Advect the picture along a curl-noise field. Loop it for smoke.
- [functionTOP](#function--functiontop) — One unary function per pixel, with a scale and offset either side.
- [glslTOP](#glsl--glsltop) — Your own shader, compiled live. WGSL or Shadertoy-style GLSL.
- [hsvadjustTOP](#hsv-adjust--hsvadjusttop) — Hue, saturation and value, optionally over one band of the wheel.
- [hsvtorgbTOP](#hsv-to-rgb--hsvtorgbtop) — The inverse of the RGB to HSV TOP.
- [inTOP](#in--intop) — A texture input on this component's node.
- [insideTOP](#inside--insidetop) — Input 2, kept only where input 1 is opaque.
- [lensdistortTOP](#lens-distort--lensdistorttop) — Bulge or pinch the frame. Negative Distort bulges (fisheye), positive corrects a bulge.
- [levelTOP](#level--leveltop) — Brightness, contrast, gamma, black/white levels.
- [limitTOP](#limit--limittop) — Clamp to a range, quantise to a step, or both.
- [lookupTOP](#lookup--lookuptop) — Input 1's brightness reads a colour out of input 2.
- [lumablurTOP](#luma-blur--lumablurtop) — Blur radius driven by input 2's brightness.
- [lumalevelTOP](#luma-level--lumaleveltop) — Brightness, contrast and gamma on luminance alone, keeping hue.
- [mathTOP](#math--mathtop) — Arithmetic on two inputs, per channel.
- [matteTOP](#matte--mattetop) — Colour from input 1, alpha from input 2.
- [mirrorTOP](#mirror--mirrortop) — Fold the image onto itself, including a radial kaleidoscope.
- [monochromeTOP](#monochrome--monochrometop) — Colour to grey, by a choice of weighting.
- [moviefileinTOP](#movie-file-in--moviefileintop) — Plays an image or a movie file.
- [moviefileoutTOP](#movie-file-out--moviefileouttop) — Records its input to a movie file. Passes the picture through.
- [multiplyTOP](#multiply--multiplytop) — The two inputs multiplied — darkens.
- [noiseTOP](#noise--noisetop) — Fractal value noise. Animate Translate Z to make it move.
- [normalmapTOP](#normal-map--normalmaptop) — A height field becomes a tangent-space normal map.
- [nullTOP](#null--nulltop) — Pass-through. A stable name to reference and to view.
- [opticalflowTOP](#optical-flow--opticalflowtop) — Local Lucas-Kanade motion from previous (input 2) to current (input 1). RG = 0.5 + gain * motion in UV/frame; B is clamped pixel magnitude divided by maxmotion. Feed a Feedback TOP targeting the source into input 2. Use matching input sizes. Estimates small translations with a 5x5 window; large motion, flat areas and changing lighting are unreliable. Lower the input resolution to reduce displacement in pixels. No pyramid or body tracking.
- [outTOP](#out--outtop) — This component's texture output.
- [outsideTOP](#outside--outsidetop) — Input 2, kept only where input 1 is not.
- [overTOP](#over--overtop) — Input 2 over input 1, by its alpha.
- [rampTOP](#ramp--ramptop) — Linear or radial gradient between two colours.
- [rectangleTOP](#rectangle--rectangletop) — A rectangle, with rounded corners and an optional border.
- [remapTOP](#remap--remaptop) — Input 2's red and green are the coordinates to read input 1 at.
- [renderTOP](#render--rendertop) — Draws Geometry components through a Camera.
- [reorderTOP](#reorder--reordertop) — Build each output channel from a channel of either input.
- [resolutionTOP](#resolution--resolutiontop) — Resamples its input to an explicit resolution.
- [rgbkeyTOP](#rgb-key--rgbkeytop) — Key out one colour by distance in RGB.
- [rgbtohsvTOP](#rgb-to-hsv--rgbtohsvtop) — Hue, saturation and value land in the red, green and blue channels.
- [screenTOP](#screen--screentop) — Multiply in the negative — lightens.
- [selectTOP](#select--selecttop) — This frame's output of another TOP, by path.
- [slopeTOP](#slope--slopetop) — The image's own gradient, as a vector, a magnitude or an angle.
- [subtractTOP](#subtract--subtracttop) — Input 2 taken away from input 1.
- [switchTOP](#switch--switchtop) — Select one of two inputs, optionally blending between them.
- [textTOP](#text--texttop) — Draws text, using a font file or the system's.
- [thresholdTOP](#threshold--thresholdtop) — Split the image in two at a level, with a soft edge.
- [tileTOP](#tile--tiletop) — Repeat the image across the frame, optionally mirrored.
- [toonTOP](#toon--toontop) — Cel shading: flatten the luminance into bands and ink the edges.
- [transformTOP](#transform--transformtop) — Translate, rotate and scale, with an extend mode.
- [underTOP](#under--undertop) — Input 1 over input 2 — Over, the other way up.
- [videodeviceinTOP](#video-device-in--videodeviceintop) — Frames from a camera or capture device.
- [voronoiTOP](#voronoi--voronoitop) — Cellular noise, as flat cells, edges or a distance field.

---

## CHOP

### Analyze — `analyzeCHOP`

Reduce each channel to one number.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Function | `function` | `average` | `average` · `maximum` · `minimum` · `rms` · `sum` · `length` · `median` |

### Animation — `animationCHOP`

Keyframed curves over time.

The keys are text — `channel time value interpolation`, one per line — so they diff cleanly and twenty evenly spaced keys are faster typed than dragged. The parameter panel draws the same data as an editable curve.

Interpolation is `constant` (hold, then jump), `linear`, `smooth` (ease in and out) or `spline` (Catmull-Rom, continuous through the keys). Outside the keyed range the curve holds its end values rather than extrapolating.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Keys | `keys` | *(multi-line)* |  |
| Play | `play` | `timeline` | `timeline` · `loop` · `hold` |
| Speed | `speed` | `1` | -4 … 4 |
| Offset | `offset` | `0` | -60 … 60 |
| Sample Rate | `rate` | `60` | 1 … 48000 |

### Audio Device In — `audiodeviceinCHOP`

Samples from an audio input, downmixed to mono.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Device (blank = default) | `device` | *(empty)* |  |
| Volume | `volume` | `1` | 0 … 8 |

### Audio Device Out — `audiodeviceoutCHOP`

Plays its first channel on an audio output.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Device (blank = default) | `device` | *(empty)* |  |
| Volume | `volume` | `1` | 0 … 2 |
| Active | `active` | `true` |  |

### Audio File In — `audiofileinCHOP`

Plays an audio file, following the timeline.

Plays RIFF/WAVE — PCM in 8, 16, 24 or 32 bits, or 32-bit float — which is what a DAW bounces, decoded in-process with no external tool. Anything else — m4a, mp3, ogg, flac, or the soundtrack of a movie file — is decoded by `ffmpeg`, the same one Movie File In uses, so it plays if ffmpeg is installed and the node says so plainly if it is not.

Playback is a function of the timeline, not a private play head: the sample at time `t` is always the file at `t × speed`. Scrubbing the timeline scrubs the audio, a loop range loops it, and a headless render reads exactly the samples the editor played.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| File (.wav) | `file` | *(empty)* |  |
| Play | `play` | `loop` | `loop` · `once` |
| Speed | `speed` | `1` | -4 … 4 |
| Volume | `volume` | `1` | 0 … 8 |

### Audio Spectrum — `audiospectrumCHOP`

Energy per frequency band, log spaced.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| FFT Size | `size` | `1024` | `256` · `512` · `1024` · `2048` · `4096` |
| Bands | `bands` | `4` | 1 … 32 |
| Gain | `gain` | `1` | 0 … 64 |
| Decibels | `decibels` | `false` |  |

### Beat — `beatCHOP`

A tempo clock: ramp, pulse, count and bar.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Tempo (BPM) | `tempo` | `120` | 20 … 300 |
| Beats Per Bar | `beatsperbar` | `4` | 1 … 32 |
| Sample Rate | `rate` | `60` | 1 … 48000 |

### Blob Track — `blobtrackCHOP`

Largest connected bright region. Channels: present, count, x, y, area, width, height, vx, vy. Coordinates and sizes are 0..1, origin top-left; velocity is frame-fractions/second. CPU readback of last frame; downsample to 160x90 first. Not person recognition or persistent identity: switching largest regions can jump. A missing/empty input returns zero channels values.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Luminance Threshold | `threshold` | `0.5` | 0 … 1 |
| Minimum Frame Fraction | `minarea` | `0.002` | 0 … 1 |

### Clock — `clockCHOP`

Wall-clock time, which keeps running when the timeline is paused.

**Inputs:** target, reference

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Sample Rate | `rate` | `60` | 1 … 48000 |
| Output | `output` | `fractions` | `units` · `fractions` · `countdown` |
| Hour Format | `hourformat` | `24` | `12` · `24` |
| Hour Adjust | `houradjust` | `0` |  |
| Start Reference | `startref` | `jan1` | `jan1` · `program` |
| Millisecond | `msec` | `true` |  |
| Second | `sec` | `true` |  |
| Minute | `min` | `true` |  |
| Hour | `hour` | `true` |  |
| AM/PM | `ampm` | `true` |  |
| Day of Week | `wday` | `true` |  |
| Day of Month | `day` | `true` |  |
| Day of Year | `yday` | `true` |  |
| Week | `week` | `true` |  |
| Month | `month` | `true` |  |
| Year | `year` | `true` |  |
| Latitude (deg) | `latitude1` | `43` |  |
| Latitude (min) | `latitude2` | `39` |  |
| N/S | `northsouth` | `north` | `north` · `south` |
| Longitude (deg) | `longitude1` | `79` |  |
| Longitude (min) | `longitude2` | `23` |  |
| E/W | `eastwest` | `west` | `east` · `west` |
| Moon Phase | `moonphase` | `false` |  |
| Sun Phase | `sunphase` | `false` |  |
| Sunrise | `sunrise` | `false` |  |
| Sunset | `sunset` | `false` |  |
| Declination | `declination` | `false` |  |

### Constant — `constantCHOP`

Fixed values as channels.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Rows | `rows` | `1` | 1 … 64 |
| Channel Name | `name0` | `chan1` |  |
| Value | `value0` | `0` |  |
| Channel Name | `name1` | `chan` |  |
| Value | `value1` | `0` |  |
| Channel Name | `name2` | `chan` |  |
| Value | `value2` | `0` |  |
| Channel Name | `name3` | `chan` |  |
| Value | `value3` | `0` |  |
| Channel Name | `name4` | `chan` |  |
| Value | `value4` | `0` |  |
| Channel Name | `name5` | `chan` |  |
| Value | `value5` | `0` |  |
| Channel Name | `name6` | `chan` |  |
| Value | `value6` | `0` |  |
| Channel Name | `name7` | `chan` |  |
| Value | `value7` | `0` |  |
| Channel Name | `name8` | `chan` |  |
| Value | `value8` | `0` |  |
| Channel Name | `name9` | `chan` |  |
| Value | `value9` | `0` |  |
| Channel Name | `name10` | `chan` |  |
| Value | `value10` | `0` |  |
| Channel Name | `name11` | `chan` |  |
| Value | `value11` | `0` |  |
| Channel Name | `name12` | `chan` |  |
| Value | `value12` | `0` |  |
| Channel Name | `name13` | `chan` |  |
| Value | `value13` | `0` |  |
| Channel Name | `name14` | `chan` |  |
| Value | `value14` | `0` |  |
| Channel Name | `name15` | `chan` |  |
| Value | `value15` | `0` |  |
| Channel Name | `name16` | `chan` |  |
| Value | `value16` | `0` |  |
| Channel Name | `name17` | `chan` |  |
| Value | `value17` | `0` |  |
| Channel Name | `name18` | `chan` |  |
| Value | `value18` | `0` |  |
| Channel Name | `name19` | `chan` |  |
| Value | `value19` | `0` |  |
| Channel Name | `name20` | `chan` |  |
| Value | `value20` | `0` |  |
| Channel Name | `name21` | `chan` |  |
| Value | `value21` | `0` |  |
| Channel Name | `name22` | `chan` |  |
| Value | `value22` | `0` |  |
| Channel Name | `name23` | `chan` |  |
| Value | `value23` | `0` |  |
| Channel Name | `name24` | `chan` |  |
| Value | `value24` | `0` |  |
| Channel Name | `name25` | `chan` |  |
| Value | `value25` | `0` |  |
| Channel Name | `name26` | `chan` |  |
| Value | `value26` | `0` |  |
| Channel Name | `name27` | `chan` |  |
| Value | `value27` | `0` |  |
| Channel Name | `name28` | `chan` |  |
| Value | `value28` | `0` |  |
| Channel Name | `name29` | `chan` |  |
| Value | `value29` | `0` |  |
| Channel Name | `name30` | `chan` |  |
| Value | `value30` | `0` |  |
| Channel Name | `name31` | `chan` |  |
| Value | `value31` | `0` |  |
| Channel Name | `name32` | `chan` |  |
| Value | `value32` | `0` |  |
| Channel Name | `name33` | `chan` |  |
| Value | `value33` | `0` |  |
| Channel Name | `name34` | `chan` |  |
| Value | `value34` | `0` |  |
| Channel Name | `name35` | `chan` |  |
| Value | `value35` | `0` |  |
| Channel Name | `name36` | `chan` |  |
| Value | `value36` | `0` |  |
| Channel Name | `name37` | `chan` |  |
| Value | `value37` | `0` |  |
| Channel Name | `name38` | `chan` |  |
| Value | `value38` | `0` |  |
| Channel Name | `name39` | `chan` |  |
| Value | `value39` | `0` |  |
| Channel Name | `name40` | `chan` |  |
| Value | `value40` | `0` |  |
| Channel Name | `name41` | `chan` |  |
| Value | `value41` | `0` |  |
| Channel Name | `name42` | `chan` |  |
| Value | `value42` | `0` |  |
| Channel Name | `name43` | `chan` |  |
| Value | `value43` | `0` |  |
| Channel Name | `name44` | `chan` |  |
| Value | `value44` | `0` |  |
| Channel Name | `name45` | `chan` |  |
| Value | `value45` | `0` |  |
| Channel Name | `name46` | `chan` |  |
| Value | `value46` | `0` |  |
| Channel Name | `name47` | `chan` |  |
| Value | `value47` | `0` |  |
| Channel Name | `name48` | `chan` |  |
| Value | `value48` | `0` |  |
| Channel Name | `name49` | `chan` |  |
| Value | `value49` | `0` |  |
| Channel Name | `name50` | `chan` |  |
| Value | `value50` | `0` |  |
| Channel Name | `name51` | `chan` |  |
| Value | `value51` | `0` |  |
| Channel Name | `name52` | `chan` |  |
| Value | `value52` | `0` |  |
| Channel Name | `name53` | `chan` |  |
| Value | `value53` | `0` |  |
| Channel Name | `name54` | `chan` |  |
| Value | `value54` | `0` |  |
| Channel Name | `name55` | `chan` |  |
| Value | `value55` | `0` |  |
| Channel Name | `name56` | `chan` |  |
| Value | `value56` | `0` |  |
| Channel Name | `name57` | `chan` |  |
| Value | `value57` | `0` |  |
| Channel Name | `name58` | `chan` |  |
| Value | `value58` | `0` |  |
| Channel Name | `name59` | `chan` |  |
| Value | `value59` | `0` |  |
| Channel Name | `name60` | `chan` |  |
| Value | `value60` | `0` |  |
| Channel Name | `name61` | `chan` |  |
| Value | `value61` | `0` |  |
| Channel Name | `name62` | `chan` |  |
| Value | `value62` | `0` |  |
| Channel Name | `name63` | `chan` |  |
| Value | `value63` | `0` |  |
| Sample Rate | `rate` | `60` | 1 … 48000 |

### Count — `countCHOP`

Count threshold crossings.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Threshold | `threshold` | `0.5` |  |
| Edge | `edge` | `rising` | `rising` · `falling` · `both` |
| Reset | `reset` | `false` |  |
| Limit | `limit` | `off` | `off` · `clamp` · `wrap` |
| Min | `min` | `0` |  |
| Max | `max` | `8` |  |

### Cross — `crossCHOP`

Blend two inputs, with an equal-power option.

**Inputs:** a, b

| Parameter | Name | Default | Range |
|---|---|---|---|
| Cross | `cross` | `0.5` | 0 … 1 |
| Curve | `curve` | `linear` | `linear` · `equalpower` · `smooth` |

### DAT to CHOP — `dattochopCHOP`

Columns or rows of a table, as channels.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Channels From | `layout` | `columns` | `columns` · `rows` |
| First Row/Column Is Names | `names` | `true` |  |

### Delay — `delayCHOP`

Play a channel back later, with optional feedback.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Delay (s) | `delay` | `0.25` | 0 … 10 |
| Mix | `mix` | `1` | 0 … 1 |
| Feedback | `feedback` | `0` | 0 … 0.95 |

### DMX Out — `dmxoutCHOP`

Sends its channels as DMX over Art-Net or sACN.

Sends each channel as a DMX slot over Art-Net or sACN (E1.31). Channel values are 0..255 (TouchDesigner-style) and clamp rather than wrap.

DMX is a state protocol, so each frame sends the current value of each channel, which is the last sample of the slice.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Protocol | `protocol` | `artnet` | `artnet` · `sacn` |
| Address (blank = broadcast / multicast) | `address` | *(empty)* |  |
| Universe | `universe` | `0` | 0 … 32767 |
| Start Channel | `start` | `1` | 1 … 512 |
| Active | `active` | `true` |  |

### Expression — `expressionCHOP`

Apply an expression to every sample. `v` is the sample.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Expression | `expr` | `v` |  |

### Fan — `fanCHOP`

Fan out an index channel to one on/off channel per value (chan0..chanN-1), or fan many channels in to the index of the first one above zero (-1 if none).

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Operation | `operation` | `fanout` | `fanout` · `fanin` |
| Channels (fan out) | `channels` | `4` | 1 … 64 |

### Filter — `filterCHOP`

Low-pass or moving-average smoothing.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Type | `type` | `lowpass` | `lowpass` · `movingaverage` |
| Width (s) | `width` | `0.2` | 0 … 4 |

### Hold — `holdCHOP`

Freeze input 1's value until input 2 fires.

**Inputs:** in, trigger

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Trigger Threshold | `threshold` | `0.5` | -10 … 10 |

### In — `inCHOP`

A channel input on this component's node.

*No inputs — this is a generator.*

### Info — `infoCHOP`

Another operator's info channels (TouchDesigner Info CHOP).

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Operator | `op` | `..` |  |

### Keyboard In — `keyboardinCHOP`

One channel per named key, 1 while held.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Keys (space separated) | `keys` | `a s d f` |  |

### Lag — `lagCHOP`

Smooth a channel, with separate rise and fall times.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Lag Up (s) | `lagup` | `0.1` | 0 … 2 |
| Lag Down (s) | `lagdown` | `0.1` | 0 … 2 |

### LFO — `lfoCHOP`

A repeating waveform over time.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Type | `type` | `sine` | `sine` · `triangle` · `ramp` · `square` · `pulse` |
| Frequency | `frequency` | `1` | 0 … 20 |
| Amplitude | `amplitude` | `1` | 0 … 4 |
| Offset | `offset` | `0` | -2 … 2 |
| Phase | `phase` | `0` | 0 … 1 |
| Pulse Width | `pulsewidth` | `0.5` | 0 … 1 |
| Channel Name | `name` | `lfo` |  |
| Sample Rate | `rate` | `60` | 1 … 48000 |

### Limit — `limitCHOP`

Clamp, wrap, fold or quantise a channel.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Type | `type` | `clamp` | `clamp` · `loop` · `zigzag` · `quantise` |
| Minimum | `min` | `0` | -10 … 10 |
| Maximum | `max` | `1` | -10 … 10 |
| Quantise Step | `step` | `0.1` | 0 … 10 |

### Logic — `logicCHOP`

Turn channels into on/off and combine them.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Convert | `convert` | `nonzero` | `off` · `nonzero` · `greater` · `less` · `equal` |
| Threshold | `threshold` | `0.5` |  |
| Invert | `invert` | `false` |  |
| Combine Channels | `combine` | `off` | `off` · `and` · `or` · `xor` |

### Math — `mathCHOP`

Combine and scale channels.

**Inputs:** a, b

| Parameter | Name | Default | Range |
|---|---|---|---|
| Channel Pre OP | `preop` | `off` | `off` · `negate` · `pos` · `root` · `square` · `inverse` |
| Combine Channels | `chanop` | `off` | `off` · `add` · `subtract` · `multiply` · `divide` · `average` · `minimum` · `maximum` · `length` |
| Combine CHOPs | `combine` | `add` | `off` · `add` · `subtract` · `multiply` · `divide` · `average` · `minimum` · `maximum` · `length` |
| Channel Post OP | `postop` | `off` | `off` · `negate` · `pos` · `root` · `square` · `inverse` |
| Match By | `match` | `index` | `index` · `name` |
| Align | `align` | `auto` | `auto` · `none` · `stretch` · `start` · `end` · `shift1` · `trim1` · `stretch1` · `trim` · `squash` |
| Integer | `integer` | `off` | `off` · `ceiling` · `floor` · `round` |
| Pre-Add | `preoff` | `0` | -4 … 4 |
| Multiply | `gain` | `1` | -4 … 4 |
| Post-Add | `offset` | `0` | -4 … 4 |
| From Range Min | `fromrange1` | `0` | -10000 … 10000 |
| From Range Max | `fromrange2` | `1` | -10000 … 10000 |
| To Range Min | `torange1` | `0` | -10000 … 10000 |
| To Range Max | `torange2` | `1` | -10000 … 10000 |
| Clamp | `clamp` | `false` |  |
| Clamp Min | `clampmin` | `0` |  |
| Clamp Max | `clampmax` | `1` |  |
| Scope | `scope` | `*` |  |

### Merge — `mergeCHOP`

Put two CHOPs' channels side by side.

**Inputs:** a, b

### MIDI In — `midiinCHOP`

Notes, velocity, pitch bend and the controls you have moved.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Port (blank = first) | `device` | *(empty)* |  |
| Notes (e.g. 36 38 42) | `notes` | *(empty)* |  |

### MIDI Out — `midioutCHOP`

Sends `nNN` channels as notes and `ccNN` channels as controls.

Channel names mirror MIDI In: `n60` is note 60, `cc74` is control 74; anything else is ignored. A note fires when its value crosses zero, with the value at that moment as velocity — while it is held, changes of value are not new notes.

Unlike DMX, MIDI is an event protocol, so only *changes* go on the wire; an unchanged control costs nothing per frame.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Port (blank = first) | `device` | *(empty)* |  |
| Channel | `channel` | `1` | 1 … 16 |
| Active | `active` | `true` |  |

### Mouse In — `mouseinCHOP`

Cursor position and buttons.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

### Noise — `noiseCHOP`

Sparse convolution noise over time; channel count comes from Channel Names.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Type | `type` | `sparse` | `sparse` · `hermite` · `harmonic` · `brownian` · `random` · `alligator` |
| Channel Names | `channelname` | `chan1` |  |
| Start | `start` | `0` |  |
| Start Unit | `startunit` | `seconds` | `seconds` · `frames` · `samples` |
| End | `end` | `10` |  |
| End Unit | `endunit` | `seconds` | `seconds` · `frames` · `samples` |
| Time Slice | `timeslice` | `false` |  |
| Period | `period` | `1` | 0 … 10 |
| Period Unit | `periodunit` | `seconds` | `seconds` · `frames` · `samples` |
| Harmonics | `harmon` | `0` | 0 … 20 |
| Harmonic Spread | `spread` | `2` | 1 … 4 |
| Roughness | `rough` | `0.5` | 0 … 1 |
| Amplitude | `amplitude` | `1` | 0 … 4 |
| Offset | `offset` | `0` | -2 … 2 |
| Seed | `seed` | `0` |  |
| Sample Rate | `rate` | `60` | 1 … 48000 |

### Null — `nullCHOP`

Pass-through. A stable name to export from.

**Inputs:** in

### OSC In — `oscinCHOP`

Incoming OSC messages, one channel per address argument.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Protocol | `protocol` | `msging` | `msging` · `multicastmsging` |
| Network Address | `netaddress` | `239.255.0.1` |  |
| Network Port | `port` | `7000` | 1 … 65535 |
| Local Address | `localaddress` | *(empty)* |  |
| OSC Address Scope | `oscaddressscope` | *(empty)* |  |
| Use Global Rate | `useglobalrate` | `true` |  |
| Default Sample Rate | `samplerate` | `60` | 1 … 48000 |

### OSC Out — `oscoutCHOP`

Sends each channel as an OSC message to the network address and port.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Protocol | `protocol` | `msging` | `msging` · `multicastmsging` · `reliablemsging` |
| Network Address | `netaddress` | `127.0.0.1` |  |
| Network Port | `port` | `7001` | 1 … 65535 |
| Local Address | `localaddress` | *(empty)* |  |
| Max Queue Size | `maxsize` | `50` | 1 … 10000 |
| Max Queue Size Unit | `maxsizeunit` | `samples` | `samples` · `frames` · `seconds` |
| Cook Every Frame | `cookalways` | `false` |  |
| Numeric Format | `numericformat` | `float` | `int` · `float` · `double` |
| Data Format | `format` | `sample` | `sample` · `timeslice` · `transpose` · `transposename` |
| Max Message Bytes | `maxbytes` | `8192` | 64 … 65536 |
| Send Events Every Cook | `sendevents` | `true` |  |

### Out — `outCHOP`

This component's channel output.

**Inputs:** in

### Panel — `panelCHOP`

Panel widget values as channels.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Panel COMPs (paths) | `ops` | *(empty)* |  |

### Parameter — `parameterCHOP`

Another operator's parameter values as channels (does not cook that operator).

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Operator | `op` | `..` |  |
| Parameters | `parameters` | `value` |  |
| Channel Name | `name` | `v` |  |

### Particle — `particleCHOP`

Stateful 2D particle emitter with velocity, gravity, lifetime and respawn. tx/ty/tz are instance positions (6 scene units across); size/life fade to zero. Emitter coordinates are normalized, origin top-left. Connect via geometryCOMP.instancechop; set sx/sy/sz to size. Active stops births; existing particles finish. Reset clears the simulation. CPU, at most 4096 particles; dt capped at 0.1s; no collisions or flocking.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Particle Count | `count` | `256` | 1 … 4096 |
| Emitter X (0..1) | `emitx` | `0.5` | 0 … 1 |
| Emitter Y (0..1, down) | `emity` | `0.5` | 0 … 1 |
| Speed (frame widths/s) | `speed` | `0.2` | 0 … 2 |
| Downward Acceleration | `gravity` | `0.2` | -2 … 2 |
| Lifetime (s) | `lifetime` | `2` | 0.1 … 20 |
| Emit | `active` | `true` |  |
| Reset | `reset` | `false` |  |

### Pattern — `patternCHOP`

A fixed-length waveform buffer.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Type | `type` | `ramp` | `ramp` · `sine` · `triangle` · `square` · `gaussian` |
| Length | `length` | `64` | 1 … 4096 |
| Periods | `periods` | `1` | 0.1 … 32 |
| Amplitude | `amplitude` | `1` | 0 … 4 |
| Offset | `offset` | `0` | -2 … 2 |
| Phase | `phase` | `0` | 0 … 1 |
| Channel Name | `name` | `chan1` |  |

### Rename — `renameCHOP`

Rename channels by pattern.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Rename Channels | `from` | `*` |  |
| To | `to` | `chan[1-]` |  |

### Resample — `resampleCHOP`

Change how many samples a buffer has, keeping its shape.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Length (samples) | `length` | `64` | 1 … 8192 |
| Interpolate | `interp` | `linear` | `linear` · `nearest` |

### Select — `selectCHOP`

Pick and rename channels.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Channel Names | `channels` | `*` |  |
| Rename To | `rename` | *(empty)* |  |

### Shuffle — `shuffleCHOP`

Rearrange the channel/sample grid — transpose, reverse, sequence.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Method | `method` | `transpose` | `transpose` · `reverse` · `swapfirstlast` · `sequence` |

### Slope — `slopeCHOP`

The rate of change of a channel.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Units | `units` | `persecond` | `persecond` · `persample` |

### SOP to CHOP — `soptochopCHOP`

Point attributes as channels, one sample per point.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Attributes (P N uv Cd) | `attrs` | `P` |  |

### Speed — `speedCHOP`

Integrate a channel: velocity becomes position.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Gain | `gain` | `1` | -4 … 4 |
| Initial Value | `initial` | `0` |  |
| Reset | `reset` | `false` |  |
| Limit | `limit` | `off` | `off` · `clamp` · `wrap` |
| Min | `min` | `0` |  |
| Max | `max` | `1` |  |

### Spring — `springCHOP`

Follow a channel like a mass on a spring: overshoot, wobble, then settle. Raise damping to stop the wobble; raise the spring constant to follow faster.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Spring Constant | `spring` | `80` | 0 … 1000 |
| Damping | `damping` | `6` | 0 … 100 |

### Switch — `switchCHOP`

Choose one of two inputs.

**Inputs:** 0, 1

| Parameter | Name | Default | Range |
|---|---|---|---|
| Index | `index` | `0` | 0 … 1 |

### Timer — `timerCHOP`

A running timer with fraction, seconds, cycles and done.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Length (s) | `length` | `10` | 0.1 … 600 |
| Cycle | `cycle` | `true` |  |
| Play | `play` | `true` |  |
| Sample Rate | `rate` | `60` | 1 … 48000 |

### TOP to CHOP — `toptochopCHOP`

Pixels as channels r g b a. `image` reads every pixel (top row first, at most 128x128) and adds u/v positions, centred, v up — instance a Geometry COMP with tx=u ty=v tz=g for a camera point cloud. Reads last frame; costs a GPU sync, so put a Resolution TOP in front.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Read | `layout` | `rows` | `rows` · `columns` · `average` · `image` |
| Row / Column | `index` | `0` | 0 … 4096 |

### Trail — `trailCHOP`

The last few seconds of each channel as one buffer, oldest sample first — draw a waveform, or lay out one instance per sample.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Window (s) | `window` | `2` | 0.05 … 60 |

### Trigger — `triggerCHOP`

An attack/decay/sustain/release envelope per channel.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Threshold | `threshold` | `0.5` |  |
| Attack (s) | `attack` | `0.05` | 0 … 2 |
| Decay (s) | `decay` | `0.2` | 0 … 4 |
| Sustain Level | `sustain` | `0` | 0 … 1 |
| Release (s) | `release` | `0.3` | 0 … 4 |

---

## COMP

### Base — `baseCOMP`

Sub-network; In and Out operators inside define inputs and outputs.

A plain component: a network inside, with **In** and **Out** operators (TOP/CHOP/SOP/DAT) defining the node's connectors. No panel UI — use `containerCOMP` only when you need a control surface.

Add `inTOP` / `outTOP` (or other family In/Out), wire internally, and the parent network sees one operator.

*No inputs — this is a generator.*

### Button — `buttonCOMP`

Toggle or momentary control; interior textCOMP + textTOP draws the label.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Label | `label` | `Button` |  |
| Value | `value` | `0` | 0 … 1 |
| Button Type | `mode` | `toggle` | `toggle` · `momentary` |
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Camera — `cameraCOMP`

A viewpoint for a Render TOP.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Translate | `translate` | `[0.0, 0.0, 5.0]` |  |
| Rotate | `rotate` | `[0.0, 0.0, 0.0]` |  |
| Look At | `lookat` | *(empty)* |  |
| Projection | `projection` | `perspective` | `perspective` · `orthographic` |
| Field of View | `fov` | `45` | 5 … 170 |
| Ortho Height | `orthosize` | `4` | 0.1 … 100 |
| Near | `near` | `0.1` | 0.001 … 10 |
| Far | `far` | `200` | 1 … 10000 |

### Container — `containerCOMP`

Parent panel for buttons, sliders, text, OP viewers, and nested containers; Children page drives align/layout.

Groups **panel** COMPs — Button, Slider, Field, and nested Containers — into a control surface on the perform output. **Layout** (x, y, width, height) is in fractions of the parent panel; **Children → Align** lays out child panels in a row, column, or grid with spacing and margins, like TouchDesigner's Container COMP.

Still holds a normal operator network (Out TOP, effects, etc.); panel layout is independent of network wiring.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| X | `x` | `0` | 0 … 1 |
| Y | `y` | `0` | 0 … 1 |
| Width | `w` | `1` | 0 … 1 |
| Height | `h` | `1` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Label | `label` | *(empty)* |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |
| View Width | `vieww` | `400` | 64 … 16384 |
| View Height | `viewh` | `300` | 64 … 16384 |
| Align | `align` | `none` | `none` · `horizlr` · `horizrl` · `verttb` · `vertbt` · `gridrows` · `gridcols` · `nodes` |
| Spacing | `spacing` | `0.02` | 0 … 0.5 |
| Max per Line | `alignmax` | `0` | 0 … 64 |
| Margin L | `marginl` | `0.02` | 0 … 0.5 |
| Margin R | `marginr` | `0.02` | 0 … 0.5 |
| Margin T | `margint` | `0.02` | 0 … 0.5 |
| Margin B | `marginb` | `0.02` | 0 … 0.5 |
| Justify Method | `justifymethod` | `individual` | `individual` · `group` |
| Justify Horizontal | `justifyh` | `off` | `off` · `left` · `center` · `right` |
| Justify Vertical | `justifyv` | `off` | `off` · `top` · `center` · `bottom` |
| Fit | `fit` | `off` | `off` · `horizontal` · `vertical` · `best` |
| Scale X | `scalex` | `1` | 0 … 8 |
| Scale Y | `scaley` | `1` | 0 … 8 |
| Offset X | `offsetx` | `0` |  |
| Offset Y | `offsety` | `0` |  |
| Crop | `crop` | `off` | `off` · `on` · `never` |

### Field — `fieldCOMP`

Editable string parameter with textCOMP/textTOP interior.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Text | `text` | *(empty)* |  |
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Geometry — `geometryCOMP`

Places a SOP in the scene, optionally instanced.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| SOP | `sop` | *(empty)* |  |
| Material | `material` | *(empty)* |  |
| Translate | `translate` | `[0.0, 0.0, 0.0]` |  |
| Rotate | `rotate` | `[0.0, 0.0, 0.0]` |  |
| Scale | `scale` | `[1.0, 1.0, 1.0]` |  |
| Instancing | `instancing` | `false` |  |
| Instance CHOP | `instancechop` | *(empty)* |  |
| Instance TOP | `instancetop` | *(empty)* |  |
| Instance Scale | `instancescale` | `1` | 0 … 10 |
| Instance Count (TOP) | `instancecount` | `1024` | 1 … 1000000 |
| Translate X Channel | `tx` | `tx` |  |
| Translate Y Channel | `ty` | `ty` |  |
| Translate Z Channel | `tz` | `tz` |  |
| Scale X Channel | `sx` | *(empty)* |  |
| Scale Y Channel | `sy` | *(empty)* |  |
| Scale Z Channel | `sz` | *(empty)* |  |
| Rotate X Channel | `rx` | *(empty)* |  |
| Rotate Y Channel | `ry` | *(empty)* |  |
| Rotate Z Channel | `rz` | *(empty)* |  |
| Red Channel | `cr` | *(empty)* |  |
| Green Channel | `cg` | *(empty)* |  |
| Blue Channel | `cb` | *(empty)* |  |
| Alpha Channel | `ca` | *(empty)* |  |

### GLSL — `glslCOMP`

Panel region filled from an interior glslTOP network (layout only until cook hook lands).

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Light — `lightCOMP`

A directional light, aimed from its position at the origin.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Translate | `translate` | `[3.0, 5.0, 4.0]` |  |
| Color | `color` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Intensity | `intensity` | `1` | 0 … 8 |

### List — `listCOMP`

Scrollable list panel — parameters match TD; rendering uses interior TOPs.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### OP Viewer — `opviewerCOMP`

Shows another operator's cooked TOP inside this panel rect.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Operator | `op` | *(empty)* |  |
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Parameter — `parameterCOMP`

Auto-layout of another operator's custom parameters (stub — build from par + TOP).

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Replicator — `replicatorCOMP`

Keeps one clone of a master component per row of a table.

Watches a template DAT and keeps one clone of its master component per data row. The first column names each replicant; any other column whose header matches a custom parameter on the master sets that parameter on the replicant — so the table *is* the population, and adding a row is adding an instance.

Replicants are ordinary clones: they follow the master's network as it is edited, and anything you place inside the replicator by hand is yours and is left alone.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Master (path of the component to copy) | `master` | *(empty)* |  |
| Template DAT (one replicant per row) | `template` | *(empty)* |  |
| First Row/Column Are Names | `byname` | `true` |  |

### Select — `selectCOMP`

Displays another panel COMP by path (stub — use opviewer + container for now).

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Slider — `sliderCOMP`

Fader control; Value parameter is the state the network reads.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Label | `label` | `Slider` |  |
| Value | `value` | `0.5` |  |
| Minimum | `min` | `0` |  |
| Maximum | `max` | `1` |  |
| Orientation | `orientation` | `horizontal` | `horizontal` · `vertical` |
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Table — `tableCOMP`

Tabular panel tied to a tableDAT (stub — interior TOP/DAT wiring).

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Text — `textCOMP`

Static or expression-driven text via interior textTOP.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Text | `text` | *(empty)* |  |
| Font Size | `fontsize` | `0.5` | 0.05 … 2 |
| X | `x` | `0.05` | 0 … 1 |
| Y | `y` | `0.05` | 0 … 1 |
| Width | `w` | `0.2` | 0 … 1 |
| Height | `h` | `0.08` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Widget — `widgetCOMP`

Reusable panel asset hook (stub — same graph-in-COMP model as other panels).

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| X | `x` | `0.02` | 0 … 1 |
| Y | `y` | `0.02` | 0 … 1 |
| Width | `w` | `0.44` | 0 … 1 |
| Height | `h` | `0.12` | 0 … 1 |
| Fixed Aspect | `fixedaspect` | `off` | `off` · `horizontal` · `vertical` |
| Aspect Ratio | `aspect` | `1` | 0.01 … 16 |
| Depth Layer | `layer` | `0` |  |
| Horizontal Mode | `hmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Left Anchor | `leftanchor` | `0` | 0 … 1 |
| Left Offset | `leftoffset` | `0` |  |
| Right Anchor | `rightanchor` | `1` | 0 … 1 |
| Right Offset | `rightoffset` | `0` |  |
| Horizontal Origin | `horigin` | `0` | 0 … 1 |
| Horizontal Fill Weight | `hfillweight` | `1` | 0 … 8 |
| Vertical Mode | `vmode` | `fixed` | `fixed` · `fill` · `anchors` |
| Bottom Anchor | `bottomanchor` | `0` | 0 … 1 |
| Bottom Offset | `bottomoffset` | `0` |  |
| Top Anchor | `topanchor` | `1` | 0 … 1 |
| Top Offset | `topoffset` | `0` |  |
| Vertical Origin | `vorigin` | `0` | 0 … 1 |
| Vertical Fill Weight | `vfillweight` | `1` | 0 … 8 |
| Parent Alignment | `alignallow` | `allow` | `allow` · `ignore` · `ignoremargin` |
| Align Order | `alignorder` | `0` |  |
| Post Offset X | `postoffsetx` | `0` |  |
| Post Offset Y | `postoffsety` | `0` |  |
| Display | `display` | `true` |  |
| Enable | `enable` | `true` |  |
| Click Through | `clickthrough` | `false` |  |
| Opacity | `opacity` | `1` | 0 … 1 |
| Background R | `bgcolorr` | `0.22` | 0 … 1 |
| Background G | `bgcolorg` | `0.24` | 0 … 1 |
| Background B | `bgcolorb` | `0.3` | 0 … 1 |
| Background Alpha | `bgalpha` | `1` | 0 … 1 |
| TOP Fill | `topfill` | `best` | `off` · `horizontal` · `vertical` · `best` · `native` · `outside` |
| Composite | `composite` | `over` | `over` · `under` · `inside` · `outside` · `add` · `subtract` · `multiply` |
| Border Over Children | `borderover` | `false` |  |

### Window — `windowCOMP`

Floating or external display for a cooked TOP (TouchDesigner Window COMP).

Shows a **cooked TOP** on a floating OS window or projector output — nothing else. Set **Operator** (`winop`) to a TOP, or a COMP (resolved to its view / Out TOP). Leave **Operator** blank to use this node's wired Out TOP.

Panel UI lives in **containerCOMP** + Perform (F1), not here. **Performance** marks this window as the project's default **output** windowCOMP (TOP path for the Output window), not the Perform UI.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Operator | `winop` | *(empty)* |  |
| Title | `title` | `Window` |  |
| Justify and Offset To | `justifyoffsetto` | `primarydisplay` | `primarydisplay` · `specifydisplay` · `alldisplays` |
| Display | `display` | `0` | 0 … 16 |
| Justify Horizontal | `justifyh` | `center` | `left` · `center` · `right` · `mouse` |
| Justify Vertical | `justifyv` | `center` | `top` · `center` · `bottom` · `mouse` |
| Offset X | `winoffsetx` | `0` | -8192 … 8192 |
| Offset Y | `winoffsety` | `0` | -8192 … 8192 |
| Opening Size | `openingsize` | `custom` | `automatic` · `fill` · `custom` · `exclusive` |
| Width | `winw` | `1280` | 64 … 16384 |
| Height | `winh` | `720` | 64 … 16384 |
| Borders | `borders` | `true` |  |
| Always on Top | `alwaysontop` | `false` |  |
| Stretch | `stretch` | `fit` | `fit` · `fill` · `stretch` · `native` |
| Cursor Visible | `cursorvisible` | `true` |  |
| Allow Minimize | `allowminimize` | `true` |  |
| V-Sync Mode | `vsyncmode` | `enabled` | `disabled` · `enabled` · `halfmonitorrate` |
| Open | `open` | `false` |  |
| Open as Perform Window | `performance` | `false` |  |
| Close on Escape | `closeescape` | `true` |  |
| Prevent Display Sleep | `blocksleep` | `false` |  |

---

## DAT

### CHOP Execute — `chopexecuteDAT`

Python callbacks when a watched channel changes or crosses a threshold.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Watch CHOP | `chop` | *(empty)* |  |
| Channels | `channels` | `*` |  |
| On Threshold | `threshold` | `0.5` | -10 … 10 |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Source | `source` | *(multi-line)* |  |

### CHOP to DAT — `choptodatDAT`

Channels as a table of numbers.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Channels As | `layout` | `columns` | `columns` · `rows` |
| Include Names | `names` | `true` |  |
| Number Format | `format` | `%.6g` |  |

### Convert — `convertDAT`

Between a table and a block of text.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Convert To | `to` | `table` | `table` · `text` |
| Delimiter | `delimiter` | `tab` | `tab` · `comma` |

### Execute — `executeDAT`

Python callbacks at the start and end of a frame.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Source | `source` | *(multi-line)* |  |

### In — `inDAT`

A data input on this component's node.

*No inputs — this is a generator.*

### JSON — `jsonDAT`

Parse JSON text into a path/value table.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| JSON Pointer (e.g. /items/0) | `pointer` | *(empty)* |  |

### Merge — `mergeDAT`

Join two DATs by rows or by columns.

**Inputs:** a, b

| Parameter | Name | Default | Range |
|---|---|---|---|
| Merge By | `how` | `rows` | `rows` · `columns` |

### Null — `nullDAT`

Pass-through. A stable name to reference.

**Inputs:** in

### Out — `outDAT`

This component's data output.

**Inputs:** in

### Panel Execute — `panelexecuteDAT`

Python callbacks when a panel value changes.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Panels | `panels` | `..` |  |
| Panel Value | `panelvalue` | `value` |  |
| Off to On | `offtoon` | `true` |  |
| On to Off | `ontooff` | `true` |  |
| Value Change | `valuechange` | `true` |  |
| While On | `whileon` | `false` |  |
| While Off | `whileoff` | `false` |  |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Source | `source` | *(multi-line)* |  |

### Parameter Execute — `parameterexecuteDAT`

Python callbacks when a watched parameter changes.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| OPs | `op` | *(empty)* |  |
| Parameters | `parameters` | `*` |  |
| Value Change | `valuechange` | `true` |  |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Source | `source` | *(multi-line)* |  |

### Script — `scriptDAT`

Rows produced by a Python script.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Source | `source` | *(multi-line)* |  |

### Select — `selectDAT`

Pick rows and columns, by name or index.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Rows (names, indices or *) | `rows` | `*` |  |
| Columns | `cols` | `*` |  |
| First Row/Column Are Names | `byname` | `true` |  |

### Sort — `sortDAT`

Sort rows by a column, numerically or as text.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Column (name or index) | `column` | `0` |  |
| Order | `order` | `ascending` | `ascending` · `descending` |
| Compare As Numbers | `numeric` | `true` |  |
| First Row Is A Header | `header` | `true` |  |

### Substitute — `substituteDAT`

Fill $name placeholders from a two-column lookup table.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Template | `template` | *(empty)* |  |
| Substitute Every Cell Instead | `table` | `false` |  |

### Table — `tableDAT`

A table of text, stored in the project file.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Contents | `text` | *(multi-line)* |  |
| Delimiter | `delimiter` | `tab` | `tab` · `comma` |

### Text — `textDAT`

A block of text.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Text | `text` | *(empty)* |  |

### Transpose — `transposeDAT`

Swap rows and columns.

**Inputs:** in

### UDP In — `udpinDAT`

Listens for datagrams; table columns address, port, message.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Local Address | `localaddress` | *(empty)* |  |
| Row Format | `format` | `permessage` | `permessage` · `perline` · `perbyte` |
| Maximum Lines | `maxlines` | `20` | 1 … 1000 |
| Bytes Column | `bytes` | `false` |  |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Source | `source` | *(multi-line)* |  |
| Port | `port` | `7000` | 1 … 65535 |

### UDP Out — `udpoutDAT`

Sends with send() / optional Send Input On Change; table shows incoming replies.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Local Address | `localaddress` | *(empty)* |  |
| Row Format | `format` | `permessage` | `permessage` · `perline` · `perbyte` |
| Maximum Lines | `maxlines` | `20` | 1 … 1000 |
| Bytes Column | `bytes` | `false` |  |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Source | `source` | *(multi-line)* |  |
| Network Address | `address` | `127.0.0.1` |  |
| Port | `port` | `7001` | 1 … 65535 |
| Send Input On Change | `sendonchange` | `false` |  |

### Web Client — `webclientDAT`

HTTP client: pulse Fetch to request in a background thread; response body is the DAT contents. WebSocket is not implemented yet.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Method | `method` | `GET` | `GET` · `POST` · `PUT` · `DELETE` |
| URL | `url` | `https://httpbin.org/get` |  |
| Headers | `headers` | *(empty)* |  |
| Request Body | `body` | *(empty)* |  |
| Fetch | `fetch` | `false` |  |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Source | `source` | *(multi-line)* |  |

### Web Server — `webserverDAT`

HTTP server: onHTTPRequest and WebSocket callbacks in the Callbacks DAT; HTTP and WebSocket on the same port.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |
| Port | `port` | `9980` | 1 … 65535 |
| Local Address | `localaddress` | *(empty)* |  |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Source | `source` | *(multi-line)* |  |

---

## MAT

### Constant — `constantMAT`

Flat colour, unaffected by lights.

**Inputs:** color

| Parameter | Name | Default | Range |
|---|---|---|---|
| Color | `basecolor` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Brightness | `emit` | `1` | 0 … 4 |

### PBR — `pbrMAT`

Base colour, metallic, roughness and emission, with an optional map.

**Inputs:** color

| Parameter | Name | Default | Range |
|---|---|---|---|
| Base Color | `basecolor` | `[0.8, 0.8, 0.85, 1.0]` |  |
| Metallic | `metallic` | `0` | 0 … 1 |
| Roughness | `roughness` | `0.4` | 0.02 … 1 |
| Emit | `emit` | `0` | 0 … 4 |

### Phong — `phongMAT`

Diffuse and a Blinn highlight, dialled by shininess.

**Inputs:** color

| Parameter | Name | Default | Range |
|---|---|---|---|
| Diffuse | `basecolor` | `[0.8, 0.8, 0.85, 1.0]` |  |
| Specular | `specular` | `0.5` | 0 … 4 |
| Shininess | `shininess` | `32` | 1 … 256 |
| Emit | `emit` | `0` | 0 … 4 |

### Point Sprite — `pointspriteMAT`

Draws every point as a camera-facing quad.

**Inputs:** color

| Parameter | Name | Default | Range |
|---|---|---|---|
| Color | `basecolor` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Size (world units) | `size` | `0.1` | 0.001 … 4 |
| Brightness | `emit` | `1` | 0 … 4 |
| Round | `round` | `true` |  |

### Wireframe — `wireframeMAT`

Draws the edges rather than the faces.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Color | `basecolor` | `[0.6, 0.9, 1.0, 1.0]` |  |
| Brightness | `emit` | `1` | 0 … 4 |

---

## SOP

### Blend — `blendSOP`

Morph between two shapes by interpolating point positions.

Interpolating point positions is the whole trick, and it only works when the two shapes agree about which point is which. They almost never do, so the interesting parameter is Match Points, which is how the correspondence gets invented.

`stretch` walks input B proportionally — point 0 of a 100-point shape pairs with point 0 of a 500-point one, point 50 with point 250 — so both surfaces are traversed end to end and a morph between two different primitives moves every point. `index` pairs point *n* with point *n*, which is right when the two are the same topology deformed two ways, and wrong otherwise.

The output keeps input A's topology and point count. Blend is a deformation of A towards B, so at 1 you have A's connectivity holding B's shape; geometry whose triangles rewired themselves halfway through would be a cut, not a morph.

**Inputs:** a, b

| Parameter | Name | Default | Range |
|---|---|---|---|
| Blend | `blend` | `0` | 0 … 1 |
| Match Points | `match` | `stretch` | `stretch` · `index` |
| Blend Normals and Color | `attributes` | `true` |  |

### Box — `boxSOP`

A box with flat-shaded faces.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Size | `size` | `[1.0, 1.0, 1.0]` |  |
| Center | `center` | `[0.0, 0.0, 0.0]` |  |

### Circle — `circleSOP`

A disc, ring or arc — filled, or a line to copy along.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Radius | `radius` | `[1.0, 1.0]` |  |
| Divisions | `divisions` | `32` | 3 … 512 |
| Arc (degrees) | `arc` | `360` | 0 … 360 |
| Fill | `fill` | `true` |  |
| Center | `center` | `[0.0, 0.0, 0.0]` |  |

### Color — `colorSOP`

Set the colour carried by every point.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Color | `color` | `[1.0, 1.0, 1.0, 1.0]` |  |

### Copy — `copySOP`

Stamp copies with a compounding transform.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Copies | `count` | `3` | 1 … 512 |
| Translate Each | `translate` | `[1.0, 0.0, 0.0]` |  |
| Rotate Each (deg) | `rotate` | `[0.0, 0.0, 0.0]` |  |
| Scale Each | `scale` | `[1.0, 1.0, 1.0]` |  |

### Facet — `facetSOP`

Recompute normals from the triangles — put after Noise or Twist so lighting follows the new shape. `unique` gives flat, faceted shading.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Normals | `mode` | `smooth` | `smooth` · `unique` |

### Grid — `gridSOP`

A flat grid of quads — the usual thing to displace.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Size | `size` | `[2.0, 2.0]` |  |
| Rows | `rows` | `10` | 2 … 512 |
| Columns | `columns` | `10` | 2 … 512 |
| Orientation | `orientation` | `xy` | `xy` · `xz` · `yz` |

### In — `inSOP`

A geometry input on this component's node.

*No inputs — this is a generator.*

### Line — `lineSOP`

A run of points between two positions.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| From | `from` | `[-1.0, 0.0, 0.0]` |  |
| To | `to` | `[1.0, 0.0, 0.0]` |  |
| Points | `points` | `2` | 2 … 4096 |

### Merge — `mergeSOP`

Combine two pieces of geometry.

**Inputs:** a, b

### Noise — `noiseSOP`

Displace points by value noise, along their normals or freely.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Amplitude | `amplitude` | `0.2` | 0 … 4 |
| Period | `period` | `1` | 0.01 … 10 |
| Offset | `offset` | `[0.0, 0.0, 0.0]` |  |
| Displace Along | `along` | `normal` | `normal` · `xyz` |

### Null — `nullSOP`

Pass-through. A stable name to reference.

**Inputs:** in

### Out — `outSOP`

This component's geometry output.

**Inputs:** in

### Sphere — `sphereSOP`

A UV sphere.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Radius | `radius` | `0.5` | 0.01 … 10 |
| Rows | `rows` | `16` | 3 … 128 |
| Columns | `columns` | `24` | 3 … 256 |
| Center | `center` | `[0.0, 0.0, 0.0]` |  |

### Torus — `torusSOP`

A torus.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Outer Radius | `radius1` | `1` | 0 … 8 |
| Inner Radius | `radius2` | `0.3` | 0 … 8 |
| Rows | `rows` | `24` | 3 … 256 |
| Columns | `columns` | `24` | 3 … 256 |
| Center | `center` | `[0.0, 0.0, 0.0]` |  |

### Transform — `transformSOP`

Translate, rotate and scale points.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Translate | `translate` | `[0.0, 0.0, 0.0]` |  |
| Rotate (deg) | `rotate` | `[0.0, 0.0, 0.0]` |  |
| Scale | `scale` | `[1.0, 1.0, 1.0]` |  |

### Tube — `tubeSOP`

A cylinder, cone or tapered tube, with optional caps.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Bottom Radius | `radius1` | `0.5` | 0 … 8 |
| Top Radius | `radius2` | `0.5` | 0 … 8 |
| Height | `height` | `2` | 0 … 16 |
| Columns | `columns` | `24` | 3 … 256 |
| Rows | `rows` | `1` | 1 … 128 |
| Caps | `caps` | `true` |  |
| Center | `center` | `[0.0, 0.0, 0.0]` |  |

### Twist — `twistSOP`

Twist, bend or taper along one axis, growing with distance from the origin. Needs enough rows along that axis to curve smoothly.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Operation | `operation` | `twist` | `twist` · `bend` · `taper` |
| Primary Axis | `axis` | `y` | `x` · `y` · `z` |
| Strength (deg, or scale for taper) | `strength` | `90` | -720 … 720 |
| Deform Length | `length` | `2` | 0.01 … 16 |

---

## TOP

### Add — `addTOP`

The two inputs summed.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Anti Alias — `antialiasTOP`

Softens luminance edges in place — FXAA as a post pass.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Strength | `strength` | `1` | 0 … 2 |

### Blur — `blurTOP`

Separable Gaussian blur.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Size (px) | `size` | `8` | 0 … 128 |

### Cache — `cacheTOP`

Holds the last frame it saw when Active is off.

Freezes its input. Turn `Active` off and the texture stops updating, which is how you hold a frame — or stop paying for a branch you are not currently looking at.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Active | `active` | `true` |  |

### Channel Mix — `channelmixTOP`

Each output channel is a weighted sum of the input's four.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Red From RGBA | `red` | `[1.0, 0.0, 0.0, 0.0]` |  |
| Green From RGBA | `green` | `[0.0, 1.0, 0.0, 0.0]` |  |
| Blue From RGBA | `blue` | `[0.0, 0.0, 1.0, 0.0]` |  |
| Alpha From RGBA | `alpha` | `[0.0, 0.0, 0.0, 1.0]` |  |

### CHOP to TOP — `choptotopTOP`

Channels as pixels: a row per channel, a column per sample.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Channel Layout | `layout` | `mono` | `mono` · `rgba` |

### Chroma Key — `chromakeyTOP`

Key out one colour, matching on hue rather than brightness.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Key Colour | `key` | `[0.0, 1.0, 0.0, 1.0]` |  |
| Tolerance | `tolerance` | `0.15` | 0 … 1 |
| Softness | `softness` | `0.1` | 0 … 1 |
| Despill | `despill` | `1` | 0 … 1 |
| Replace Rather Than Cut | `replace` | `false` |  |
| Replacement | `replacement` | `[0.0, 0.0, 0.0, 1.0]` |  |

### Circle — `circleTOP`

An antialiased disc or ellipse.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Centre | `centre` | `[0.5, 0.5]` |  |
| Radius | `radius` | `[0.3, 0.3]` |  |
| Softness | `softness` | `0` | 0 … 1 |
| Correct Aspect | `aspect` | `true` |  |
| Fill | `fill` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Background | `background` | `[0.0, 0.0, 0.0, 0.0]` |  |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Composite — `compositeTOP`

Blend two inputs. Input 2 is composited over input 1.

**Inputs:** base, over

| Parameter | Name | Default | Range |
|---|---|---|---|
| Operation | `operation` | `over` | `over` · `add` · `multiply` · `screen` · `difference` · `subtract` · `maximum` · `minimum` · `under` · `inside` · `outside` · `cross` |
| Opacity | `opacity` | `1` | 0 … 1 |

### Constant — `constantTOP`

A flat colour at a chosen resolution.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Color | `color` | `[0.0, 0.0, 0.0, 1.0]` |  |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Convolve — `convolveTOP`

An arbitrary 3x3 kernel: sharpen, blur, edge detect.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Kernel Row 1 | `row0` | `[0.0, 0.0, 0.0]` |  |
| Kernel Row 2 | `row1` | `[0.0, 1.0, 0.0]` |  |
| Kernel Row 3 | `row2` | `[0.0, 0.0, 0.0]` |  |
| Spread (px) | `spread` | `1` | 0 … 32 |
| Normalize | `normalize` | `true` |  |
| Scale | `scale` | `1` | -4 … 4 |
| Offset | `offset` | `0` | -1 … 1 |

### Corner Pin — `cornerpinTOP`

Drag the image's four corners anywhere — a perspective warp.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Bottom Left | `bottomleft` | `[0.0, 0.0]` |  |
| Bottom Right | `bottomright` | `[1.0, 0.0]` |  |
| Top Left | `topleft` | `[0.0, 1.0]` |  |
| Top Right | `topright` | `[1.0, 1.0]` |  |
| Extend | `extend` | `zero` | `zero` · `hold` · `repeat` · `mirror` |

### Crop — `cropTOP`

Keep a rectangle of the source, at an explicit output size.

The region you keep is resampled to `Resolution W`/`H`, so this is a crop *and* a resize in one node. For a crop that keeps the source's pixel scale, set the resolution to the region's size in pixels — a half-width crop of a 1920 wide source wants 960.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Left | `left` | `0` | 0 … 1 |
| Right | `right` | `1` | 0 … 1 |
| Bottom | `bottom` | `0` | 0 … 1 |
| Top | `top` | `1` | 0 … 1 |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Cross — `crossTOP`

Each input survives where the other is not.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Difference — `differenceTOP`

The absolute difference between the inputs.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Displace — `displaceTOP`

Offsets input 1's lookup by channels of input 2.

**Inputs:** source, displace

| Parameter | Name | Default | Range |
|---|---|---|---|
| Amount | `amount` | `0.1` | -1 … 1 |
| Source X | `sourcex` | `r` | `r` · `g` · `b` · `a` · `luminance` |
| Source Y | `sourcey` | `g` | `r` · `g` · `b` · `a` · `luminance` |
| Offset | `offset` | `-0.5` | -1 … 1 |
| Extend | `extend` | `hold` | `zero` · `hold` · `repeat` · `mirror` |

### Dither — `ditherTOP`

Quantise to few levels, with an ordered or noise dither.

Quantising alone gives flat bands. Dithering adds a pattern *before* the quantiser so the rounding error alternates between neighbouring pixels and the eye averages it back into the tone that was there — which is how two colours can look like a gradient.

Which pattern is the whole look: `bayer4` and `bayer8` are the ordered crosshatch of early games and newsprint, `noise` is closer to film grain, and `none` is hard posterisation with no dither at all. Pixel Size above 1 enlarges the matrix, which is what makes it read as chunky rather than as texture.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Levels | `levels` | `4` | 2 … 64 |
| Pattern | `pattern` | `bayer4` | `bayer4` · `bayer8` · `noise` · `none` |
| Strength | `strength` | `1` | 0 … 1 |
| Pixel Size | `scale` | `1` | 1 … 32 |
| Monochrome | `monochrome` | `false` |  |

### Edge — `edgeTOP`

Sobel edge detection.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Strength | `strength` | `1` | 0 … 8 |
| Width (px) | `width` | `1` | 0 … 16 |
| Direction | `direction` | `both` | `both` · `horizontal` · `vertical` |
| Keep Colour | `keepcolor` | `0` | 0 … 1 |
| Edge Colour | `color` | `[1.0, 1.0, 1.0, 1.0]` |  |

### Emboss — `embossTOP`

A directional difference, sat at mid grey.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Direction | `direction` | `45` | -180 … 180 |
| Width (px) | `width` | `1` | 0 … 32 |
| Strength | `strength` | `4` | 0 … 32 |
| Mix With Source | `mix` | `0` | 0 … 1 |

### Feedback — `feedbackTOP`

Last frame's output of the Target TOP.

Reads the target TOP's output from the **previous** frame. That is what lets a feedback loop exist without a cycle in the cook graph — the Select TOP, which reads the current frame, would be a cycle and is rejected.

Point `Target TOP` at the node whose output you want to feed back, wire this node's output into that chain, and put a Level TOP in between to decay it. Without the decay the loop saturates within a second.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Target TOP | `target` | *(empty)* |  |

### Fit — `fitTOP`

Resample into a new resolution: fill, fit, stretch or native.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Fit | `mode` | `fit` | `fill` · `fit` · `stretch` · `native` |
| Background | `background` | `[0.0, 0.0, 0.0, 0.0]` |  |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Flip — `flipTOP`

Mirror the image about an axis, or transpose it.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Flip Horizontally | `flipx` | `true` |  |
| Flip Vertically | `flipy` | `false` |  |
| Transpose | `transpose` | `false` |  |

### Flow — `flowTOP`

Advect the picture along a curl-noise field. Loop it for smoke.

One Flow TOP is a warp: every pixel reads from upstream of a curl-noise field, so the picture leans. The look people mean by *flow* — smoke, ink in water, drifting abstraction — is that warp **in a loop**, where each frame advects the last one a little further:

`source -> flow1 -> compositeTOP` with a Feedback TOP targeting the composite and wired back into it.

The field is the curl of a noise field rather than the noise itself, and that is not a detail. A curl is divergence-free: it swirls without compressing the image into a point or tearing a hole in it, which a plain noise vector field does within a second of being looped. Wire a TOP into the second input and turn on Steer From Input 2 to drive the flow with a picture — a Ramp for a wind direction, a camera difference for something that follows you.

**Inputs:** in, field

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Amount (px) | `amount` | `6` | 0 … 128 |
| Field Scale | `scale` | `3` | 0.1 … 32 |
| Field Speed | `speed` | `0.3` | 0 … 8 |
| Steer From Input 2 | `usefield` | `false` |  |
| Field Mix | `fieldmix` | `1` | 0 … 1 |

### Function — `functionTOP`

One unary function per pixel, with a scale and offset either side.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Function | `function` | `none` | `none` · `log` · `exp` · `sqrt` · `square` · `reciprocal` · `invert` · `sin` · `cos` · `atan` · `abs` · `sign` |
| Pre Multiply | `premultiply` | `1` | -4 … 4 |
| Pre Add | `preadd` | `0` | -4 … 4 |
| Post Multiply | `postmultiply` | `1` | -4 … 4 |
| Post Add | `postadd` | `0` | -4 … 4 |
| Include Alpha | `alpha` | `false` |  |

### GLSL — `glslTOP`

Your own shader, compiled live. WGSL or Shadertoy-style GLSL.

Write a fragment body; the boilerplate is supplied. In WGSL, `in.uv`, `U.time.x`, `U.res` and `sample0(uv)`/`sample1(uv)` are in scope, along with `U.p0..U.p3` from the four Uniform parameters. In GLSL a Shadertoy `mainImage` with `iTime` and `iResolution` runs unmodified, and the two inputs are `iChannel0`/`iChannel1` — sampled with `texture()`, and already declared, so declaring them yourself is an error. TouchDesigner's `sTD2DInputs` is not a name here.

Sources are validated before the GPU sees them, so a typo gives a line number and the last shader that compiled keeps running — the patch never goes black mid-edit. *Import ISF…* loads a published ISF shader and turns its inputs into parameters on this node.

**Inputs:** in0, in1

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Language | `language` | `wgsl` | `wgsl` · `glsl` |
| Source | `source` | *(multi-line)* |  |
| Uniform 1 | `uniform1` | `[1.0, 0.6, 0.2, 1.0]` |  |
| Uniform 2 | `uniform2` | `[0.0, 0.0, 0.0, 0.0]` |  |
| Uniform 3 | `uniform3` | `[0.0, 0.0, 0.0, 0.0]` |  |
| Uniform 4 | `uniform4` | `[0.0, 0.0, 0.0, 0.0]` |  |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### HSV Adjust — `hsvadjustTOP`

Hue, saturation and value, optionally over one band of the wheel.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Hue Shift | `hue` | `0` | -0.5 … 0.5 |
| Saturation | `saturation` | `1` | 0 … 4 |
| Value | `value` | `1` | 0 … 4 |
| Contrast | `contrast` | `1` | 0 … 4 |
| Range Centre | `rangecentre` | `0` | 0 … 1 |
| Range Width | `rangewidth` | `1` | 0 … 1 |

### HSV to RGB — `hsvtorgbTOP`

The inverse of the RGB to HSV TOP.

**Inputs:** in

### In — `inTOP`

A texture input on this component's node.

*No inputs — this is a generator.*

### Inside — `insideTOP`

Input 2, kept only where input 1 is opaque.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Lens Distort — `lensdistortTOP`

Bulge or pinch the frame. Negative Distort bulges (fisheye), positive corrects a bulge.

The sign of `Distort` is the whole operator and is easy to get backwards. NEGATIVE bulges the middle out — the fisheye, the GoPro look, barrel distortion. POSITIVE pinches it back in, which is the *correction* for footage that already bulges. So undoing a GoPro is a positive number, not a negative one.

`Distort Squared` is the higher-order term and only shows up at the very edge of a wide lens; leave it at zero until the corners are still wrong with `Distort` right. Correcting a bulge leaves black corners where the image used to be — `Scale` above 1 crops them off.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Distort (- bulges, + corrects) | `k1` | `0` | -1 … 1 |
| Distort Squared | `k2` | `0` | -1 … 1 |
| Scale | `scale` | `1` | 0.1 … 4 |
| Center | `center` | `[0.5, 0.5]` |  |
| Extend | `extend` | `zero` | `zero` · `hold` · `repeat` · `mirror` |

### Level — `levelTOP`

Brightness, contrast, gamma, black/white levels.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Brightness | `brightness` | `1` | 0 … 4 |
| Contrast | `contrast` | `1` | 0 … 4 |
| Gamma | `gamma` | `1` | 0.1 … 4 |
| Opacity | `opacity` | `1` | 0 … 1 |
| Black Level | `blacklevel` | `0` | 0 … 1 |
| White Level | `whitelevel` | `1` | 0 … 1 |
| Invert | `invert` | `0` | 0 … 1 |

### Limit — `limitTOP`

Clamp to a range, quantise to a step, or both.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Mode | `mode` | `clamp` | `clamp` · `quantize` · `both` |
| Minimum | `min` | `0` | -2 … 2 |
| Maximum | `max` | `1` | -2 … 2 |
| Quantize Step | `step` | `0.25` | 0.001 … 1 |
| Include Alpha | `alpha` | `false` |  |

### Lookup — `lookupTOP`

Input 1's brightness reads a colour out of input 2.

**Inputs:** index, table

| Parameter | Name | Default | Range |
|---|---|---|---|
| Index By | `source` | `luminance` | `luminance` · `red` · `green` · `blue` · `alpha` |

### Luma Blur — `lumablurTOP`

Blur radius driven by input 2's brightness.

**Inputs:** source, blur

| Parameter | Name | Default | Range |
|---|---|---|---|
| White Radius (px) | `white` | `16` | 0 … 128 |
| Black Radius (px) | `black` | `0` | 0 … 128 |
| Radius From | `source` | `luminance` | `luminance` · `alpha` · `red` |

### Luma Level — `lumalevelTOP`

Brightness, contrast and gamma on luminance alone, keeping hue.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Brightness | `brightness` | `1` | 0 … 4 |
| Contrast | `contrast` | `1` | 0 … 4 |
| Gamma | `gamma` | `1` | 0.1 … 4 |
| Opacity | `opacity` | `1` | 0 … 1 |
| Black Level | `blacklevel` | `0` | 0 … 1 |
| White Level | `whitelevel` | `1` | 0 … 1 |

### Math — `mathTOP`

Arithmetic on two inputs, per channel.

**Inputs:** a, b

| Parameter | Name | Default | Range |
|---|---|---|---|
| Operation | `operation` | `add` | `add` · `subtract` · `multiply` · `divide` · `minimum` · `maximum` · `difference` · `power` |
| Input 1 Gain | `gain1` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Input 2 Gain | `gain2` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Offset | `offset` | `[0.0, 0.0, 0.0, 0.0]` |  |

### Matte — `matteTOP`

Colour from input 1, alpha from input 2.

**Inputs:** source, matte

| Parameter | Name | Default | Range |
|---|---|---|---|
| Matte From | `source` | `alpha` | `alpha` · `luminance` · `red` |
| Invert | `invert` | `false` |  |
| Premultiply | `premultiply` | `false` |  |

### Mirror — `mirrorTOP`

Fold the image onto itself, including a radial kaleidoscope.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Mode | `mode` | `horizontal` | `horizontal` · `vertical` · `quad` · `radial` |
| Segments | `segments` | `6` | 2 … 32 |
| Angle | `angle` | `0` | -180 … 180 |
| Centre | `centre` | `[0.5, 0.5]` |  |

### Monochrome — `monochromeTOP`

Colour to grey, by a choice of weighting.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Mode | `mode` | `luminance` | `luminance` · `average` · `maximum` · `minimum` · `red` · `green` · `blue` · `alpha` |
| Opacity | `opacity` | `1` | 0 … 1 |

### Movie File In — `moviefileinTOP`

Plays an image or a movie file.

Still images — PNG, JPEG, WebP, BMP, TGA, TIFF — are decoded in-process, with no external tool involved. Everything that moves goes through an `ffmpeg` subprocess, so mp4, mov, mkv, webm, avi and animated GIF all play if ffmpeg is installed. It is looked for on the PATH and in the usual install directories, so a double-clicked app finds it too, and the node says so plainly when it cannot.

Playback is a function of the timeline, not a private play head: the frame shown at time `t` is always the file at `t × speed`. Scrubbing the timeline scrubs the movie, the loop range loops it, and a headless `otd render` writes exactly the frames the editor showed. Seeking backwards restarts the decode at the new time, which is why a scrub is a real scrub rather than a rewind.

The picture's own size wins: `Fallback W`/`H` is only what to show before the first frame arrives, or if the file cannot be read.

`Speed` tracks the timeline exactly at 1× and 2×. Past that it falls behind, and the reason is the transport rather than the decoder: frames cross a pipe as raw RGBA, so 1138×640 is 2.9 MB each, and asking for 8× is asking for well over a gigabyte a second through it. Scrubbing is unaffected — a jump seeks rather than races.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| File | `file` | `apple.png` |  |
| Reload | `reload` | `false` |  |
| Play Mode | `playmode` | `sequential` | `locked` · `specify` · `sequential` · `timecodeop` |
| Play | `play` | `true` |  |
| Speed | `speed` | `1` | -10 … 10 |
| Cue | `cue` | `false` |  |
| Cue Pulse | `cuepulse` | `false` |  |
| Cue Point | `cuepoint` | `0` |  |
| Cue Point Unit | `cuepointunit` | `indices` | `indices` · `frames` · `seconds` · `fraction` |
| Cue Behavior | `cuebehavior` | `repeat` | `repeat` · `play` |
| Index | `index` | `0` |  |
| Index Unit | `indexunit` | `indices` | `indices` · `frames` · `seconds` · `fraction` |
| Loop Crossfade | `loopcrossfade` | `0` | 0 … 60 |
| Loop Crossfade Unit | `loopcrossfadeunit` | `seconds` | `indices` · `frames` · `seconds` · `fraction` |
| Step Size | `stepsize` | `1` | 1 … 1000 |
| Audio Loop | `audioloop` | `silence` | `silence` · `fade` · `match` |
| Image Sequence Indexing | `imageindexing` | `zero` | `zero` · `filename` |
| Timecode Mode | `timecodemode` | `absolute` | `absolute` · `usefiletimecode` |
| Timecode Object/CHOP/DAT | `timecodeop` | *(empty)* |  |
| Active | `active` | `true` |  |
| Interpolate Frames | `interp` | `false` |  |
| Deinterlace | `deinterlace` | `off` | `off` · `even` · `odd` · `bob` |
| Field Precedence | `precedence` | `even` | `even` · `odd` |
| Bottom Half is Alpha (AAA) | `bottomhalfalpha` | `false` |  |
| Pre-Multiply RGB by Alpha | `premultrgbbyalpha` | `automatic` | `off` · `on` · `automatic` |
| Input Color Space | `inputcolorspace` | `automatic` | `automatic` · `srgb` · `srgblinear` · `rec601ntsc` · `rec709` · `rec2020` · `rec2020st2084pq` · `rec2020hlg` · `dcip3` · `dcip3d60` · `displayp3d65` · `displayp3d65linear` · `aces2065-1` · `acescg` · `acesproxy` · `passthrough` · `rec709gamma24` |
| Input Reference White | `inputreferencewhite` | `default` | `default` · `sdr` · `hdr` · `ui` |
| HLG Peak Nits | `hlgpeaknits` | `1000` | 100 … 10000 |
| Colored Bottom Right Square | `coloredbottomright` | `true` |  |
| Decode Pixel Format | `decodepixelformat` | `fastest` | `fastest` · `best` |
| Fallback W | `resw` | `1280` | 1 … 16384 |
| Fallback H | `resh` | `720` | 1 … 16384 |
| Trim | `trim` | `false` |  |
| Trim Start | `tstart` | `0` |  |
| Trim Start Unit | `tstartunit` | `indices` | `indices` · `frames` · `seconds` · `fraction` |
| Trim End | `tend` | `1` |  |
| Trim End Unit | `tendunit` | `fraction` | `indices` · `frames` · `seconds` · `fraction` |
| Extend Left | `extendleft` | `hold` | `hold` · `cycle` · `mirror` · `black` · `zero` |
| Extend Right | `extendright` | `cycle` | `hold` · `cycle` · `mirror` · `black` · `zero` |
| Override Sample Rate | `overridesample` | `false` |  |
| Sample Rate | `samplerate` | `30` | 1 … 240 |
| Pre-Read Frames | `prereadframes` | `16` | 0 … 120 |
| Frame Read Timeout (ms) | `frametimeout` | `5000` | 0 … 60000 |
| Frame Timeout Strategy | `frametimeoutstrat` | `playback` | `keep` · `playback` · `seeking` |
| Always Load Initial Frame | `alwaysloadinitial` | `true` |  |
| File Open Timeout (ms) | `opentimeout` | `10000` | 0 … 60000 |
| Pre-Download HTTP Addresses | `predownloadhttp` | `false` |  |
| Update Image | `updateimage` | `true` |  |
| Max Decode CPUs (0=all) | `maxdecodecpus` | `0` | 0 … 64 |
| High Performance Read | `highperfread` | `false` |  |
| High Performance Read Factor | `highperfreadfactor` | `1` | 1 … 8 |
| Hardware Decode | `hwdecode` | `true` |  |
| Callbacks DAT | `callbacks` | *(empty)* |  |
| Output Resolution | `outputresolution` | `useinput` | `useinput` · `eighth` · `quarter` · `half` · `2x` · `4x` · `8x` · `fit` · `limit` · `custom` |
| W | `resolutionw` | `1280` | 1 … 16384 |
| H | `resolutionh` | `720` | 1 … 16384 |
| Use Global Res Multiplier | `resmult` | `true` |  |
| Output Aspect | `outputaspect` | `useinput` | `useinput` · `resolution` · `custom` |
| Aspect1 | `aspect1` | `1` |  |
| Aspect2 | `aspect2` | `1` |  |
| Aspect Menu | `armenu` | `16:9` | `16:9` · `4:3` · `1:1` · `2.39:1` |
| Viewer Smoothness | `filtertype` | `linear` | `nearest` · `linear` · `mipmap` |
| Fill Viewer | `fillmode` | `nativeres` | `useinput` · `fill` · `width` · `height` · `best` · `outside` · `nativeres` |
| Passes | `npasses` | `1` | 1 … 32 |
| Channel Mask | `chanmask` | `rgba` |  |
| Pixel Format | `format` | `useinput` | `useinput` · `rgba8fixed` · `srgba8fixed` · `rgba16float` · `rgba32float` · `rgb10a2fixed` · `rgba16fixed` · `rgba11float` · `rgb16float` |

### Movie File Out — `moviefileoutTOP`

Records its input to a movie file. Passes the picture through.

**Inputs:** in

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Type | `type` | `movie` | `movie` · `image` · `imagesequence` · `stopframemovie` |
| Video Codec | `codec` | `h264` | `h264` · `h265` · `prores` |
| Image File Type | `imagefiletype` | `png` | `png` · `jpeg` · `tiff` · `bmp` |
| Unique Suffix | `uniquesuff` | `false` |  |
| N | `n` | `1` | 0 … 1000000 |
| Leading Zeros Digits | `leadingzerosdigits` | `4` | 1 … 12 |
| File | `file` | `out.mp4` |  |
| Movie FPS | `fps` | `60` | 1 … 240 |
| Quality | `quality` | `75` | 0 … 100 |
| Limit Length | `limitlength` | `false` |  |
| Length | `length` | `600` | 0 … 1000000 |
| Length Unit | `lengthunit` | `frames` | `indices` · `frames` · `seconds` |
| Record | `record` | `false` |  |
| Pause | `pause` | `false` |  |
| Add Frame | `addframe` | `false` |  |
| Output Resolution | `outputresolution` | `useinput` | `useinput` · `eighth` · `quarter` · `half` · `2x` · `4x` · `8x` · `fit` · `limit` · `custom` |
| W | `resolutionw` | `1280` | 1 … 16384 |
| H | `resolutionh` | `720` | 1 … 16384 |
| Viewer Smoothness | `filtertype` | `linear` | `nearest` · `linear` · `mipmap` |
| Fill Viewer | `fillmode` | `nativeres` | `useinput` · `fill` · `width` · `height` · `best` · `outside` · `nativeres` |

### Multiply — `multiplyTOP`

The two inputs multiplied — darkens.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Noise — `noiseTOP`

Fractal value noise. Animate Translate Z to make it move.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Period | `period` | `0.25` | 0.01 … 4 |
| Harmonics | `harmonics` | `3` | 1 … 10 |
| Roughness | `roughness` | `0.5` | 0 … 1 |
| Exponent | `exponent` | `1` | 0.1 … 8 |
| Translate | `translate` | `[0.0, 0.0, 0.0]` |  |
| Monochrome | `monochrome` | `true` |  |
| Amplitude | `amplitude` | `1` | 0 … 4 |
| Offset | `offset` | `0` | -1 … 1 |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Normal Map — `normalmapTOP`

A height field becomes a tangent-space normal map.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Height From | `source` | `luminance` | `luminance` · `alpha` · `red` |
| Strength | `strength` | `4` | 0 … 64 |
| Flip Green | `flipgreen` | `false` |  |

### Null — `nullTOP`

Pass-through. A stable name to reference and to view.

**Inputs:** in

### Optical Flow — `opticalflowTOP`

Local Lucas-Kanade motion from previous (input 2) to current (input 1). RG = 0.5 + gain * motion in UV/frame; B is clamped pixel magnitude divided by maxmotion. Feed a Feedback TOP targeting the source into input 2. Use matching input sizes. Estimates small translations with a 5x5 window; large motion, flat areas and changing lighting are unreliable. Lower the input resolution to reduce displacement in pixels. No pyramid or body tracking.

**Inputs:** current, previous

| Parameter | Name | Default | Range |
|---|---|---|---|
| Vector Gain | `gain` | `1` | 0 … 20 |
| Noise Regularization | `regularization` | `0.0001` | 0.000001 … 0.1 |
| Maximum Motion (px/frame) | `maxmotion` | `4` | 0.1 … 16 |

### Out — `outTOP`

This component's texture output.

**Inputs:** in

### Outside — `outsideTOP`

Input 2, kept only where input 1 is not.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Over — `overTOP`

Input 2 over input 1, by its alpha.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Ramp — `rampTOP`

Linear or radial gradient between two colours.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Type | `type` | `horizontal` | `horizontal` · `vertical` · `radial` |
| Phase | `phase` | `0` | 0 … 1 |
| Color 1 | `color1` | `[0.0, 0.0, 0.0, 1.0]` |  |
| Color 2 | `color2` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Rectangle — `rectangleTOP`

A rectangle, with rounded corners and an optional border.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Centre | `centre` | `[0.5, 0.5]` |  |
| Size | `size` | `[0.5, 0.5]` |  |
| Softness | `softness` | `0` | 0 … 0.5 |
| Corner Radius | `corner` | `0` | 0 … 0.5 |
| Border Width | `border` | `0` | 0 … 0.5 |
| Border Grey | `bordercolor` | `0` | 0 … 1 |
| Fill | `fill` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Background | `background` | `[0.0, 0.0, 0.0, 0.0]` |  |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Remap — `remapTOP`

Input 2's red and green are the coordinates to read input 1 at.

**Inputs:** source, map

| Parameter | Name | Default | Range |
|---|---|---|---|
| Amount | `amount` | `1` | 0 … 1 |
| Extend | `extend` | `zero` | `zero` · `hold` · `repeat` · `mirror` |
| Flip V | `flipv` | `false` |  |

### Render — `renderTOP`

Draws Geometry components through a Camera.

Depth output is the same draw with the shading skipped, writing distance from the camera as grey — near white, far black, so an unwritten background is black and the result multiplies straight into a composite as a mask.

It is metric distance between Depth Near and Depth Far, not the depth buffer's own value. That one is `1/z` shaped by the projection and puts almost all of its precision in the first few units, so everything past arm's length reads the same white and anything downstream sees a flat card. Set the near and far to the part of the scene you care about; they are what decide whether the pass looks like anything.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Geometry | `geometry` | *(empty)* |  |
| Camera | `camera` | *(empty)* |  |
| Light | `light` | *(empty)* |  |
| Background | `background` | `[0.0, 0.0, 0.0, 1.0]` |  |
| Ambient | `ambient` | `0.12` | 0 … 1 |
| Wireframe | `wireframe` | `false` |  |
| Cull | `cull` | `back` | `back` · `front` · `none` |
| Output | `output` | `color` | `color` · `depth` |
| Depth Near | `depthnear` | `0.1` | 0 … 1000 |
| Depth Far | `depthfar` | `20` | 0.01 … 10000 |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Reorder — `reorderTOP`

Build each output channel from a channel of either input.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Red | `red` | `input1 r` | `input1 r` · `input1 g` · `input1 b` · `input1 a` · `input2 r` · `input2 g` · `input2 b` · `input2 a` · `input1 luma` · `input2 luma` · `one` · `zero` |
| Green | `green` | `input1 g` | `input1 r` · `input1 g` · `input1 b` · `input1 a` · `input2 r` · `input2 g` · `input2 b` · `input2 a` · `input1 luma` · `input2 luma` · `one` · `zero` |
| Blue | `blue` | `input1 b` | `input1 r` · `input1 g` · `input1 b` · `input1 a` · `input2 r` · `input2 g` · `input2 b` · `input2 a` · `input1 luma` · `input2 luma` · `one` · `zero` |
| Alpha | `alpha` | `input1 a` | `input1 r` · `input1 g` · `input1 b` · `input1 a` · `input2 r` · `input2 g` · `input2 b` · `input2 a` · `input1 luma` · `input2 luma` · `one` · `zero` |

### Resolution — `resolutionTOP`

Resamples its input to an explicit resolution.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Filter | `filter` | `linear` | `linear` · `nearest` |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### RGB Key — `rgbkeyTOP`

Key out one colour by distance in RGB.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Key Color | `color` | `[0.0, 1.0, 0.0, 1.0]` |  |
| Tolerance | `tolerance` | `0.2` | 0 … 1.732 |
| Softness | `softness` | `0.1` | 0 … 1 |
| Output | `output` | `keyed` | `keyed` · `matte` · `inverted matte` |

### RGB to HSV — `rgbtohsvTOP`

Hue, saturation and value land in the red, green and blue channels.

**Inputs:** in

### Screen — `screenTOP`

Multiply in the negative — lightens.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Select — `selectTOP`

This frame's output of another TOP, by path.

Reads another TOP's output from the **current** frame, so the source cooks first. Use it to fan a texture out to several branches without drawing wires across the whole network. For a loop, use Feedback instead: a Select pointing back at its own chain is a cycle and is refused.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| TOP | `top` | *(empty)* |  |

### Slope — `slopeTOP`

The image's own gradient, as a vector, a magnitude or an angle.

The `xy` output is red = d/dx, green = d/dy, centred on 0.5 — which is exactly the layout a Displace TOP wants in its second input. So `noise1 -> slope1 -> displace1` warps a picture along a noise field with no shader anywhere in it.

`magnitude` is an edge detector that does not care which way the edge runs, and `direction` is the angle as a hue, which is mostly useful to look at while you work out why a warp is going the wrong way.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Output | `mode` | `xy` | `xy` · `magnitude` · `direction` |
| Strength | `strength` | `1` | 0 … 32 |

### Subtract — `subtractTOP`

Input 2 taken away from input 1.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Switch — `switchTOP`

Select one of two inputs, optionally blending between them.

**Inputs:** 0, 1

| Parameter | Name | Default | Range |
|---|---|---|---|
| Index | `index` | `0` | 0 … 1 |
| Blend | `blend` | `false` |  |

### Text — `textTOP`

Draws text, using a font file or the system's.

*No inputs — this is a generator.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Text | `text` | `OpenTouchDesigner` |  |
| Font File | `font` | *(empty)* |  |
| Size (px) | `size` | `48` | 4 … 512 |
| Line Spacing | `linespacing` | `1.2` | 0.5 … 4 |
| Horizontal Align | `halign` | `centre` | `left` · `centre` · `right` |
| Vertical Align | `valign` | `centre` | `top` · `centre` · `bottom` |
| Word Wrap | `wrap` | `true` |  |
| Color | `color` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Background | `background` | `[0.0, 0.0, 0.0, 0.0]` |  |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

### Threshold — `thresholdTOP`

Split the image in two at a level, with a soft edge.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Threshold | `threshold` | `0.5` | 0 … 1 |
| Softness | `softness` | `0.02` | 0 … 1 |
| Compare | `source` | `luminance` | `luminance` · `maximum` · `alpha` |
| Invert | `invert` | `false` |  |
| Below Colour | `below` | `[0.0, 0.0, 0.0, 1.0]` |  |
| Above Colour | `above` | `[1.0, 1.0, 1.0, 1.0]` |  |

### Tile — `tileTOP`

Repeat the image across the frame, optionally mirrored.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Repeat | `repeat` | `[2.0, 2.0]` |  |
| Offset | `offset` | `[0.0, 0.0]` |  |
| Mirror | `mirror` | `false` |  |
| Row Shift | `rowshift` | `0` | 0 … 1 |
| Column Shift | `columnshift` | `0` | 0 … 1 |

### Toon — `toonTOP`

Cel shading: flatten the luminance into bands and ink the edges.

Cel shading is two ideas that only look like one, and this operator is both because they want to share a threshold — ink drawn where the bands already change is invisible, and ink drawn anywhere else is a mess.

Posterising the *luminance*, and only the luminance, collapses a smooth gradient into flat steps while leaving hue alone, which is what keeps the result looking painted rather than colour-crushed. Flattening luminance washes colour out, so Saturation defaults above 1 to put it back. The ink is a Sobel edge multiplied over the top: multiplied rather than added, because added lines glow, which is the opposite of a drawn line.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Bands | `bands` | `4` | 2 … 32 |
| Ink Strength | `edge` | `2` | 0 … 16 |
| Ink Width (px) | `edgewidth` | `1` | 0 … 8 |
| Saturation | `saturation` | `1.3` | 0 … 4 |
| Ink Color | `inkcolor` | `[0.0, 0.0, 0.0, 1.0]` |  |

### Transform — `transformTOP`

Translate, rotate and scale, with an extend mode.

**Inputs:** in

| Parameter | Name | Default | Range |
|---|---|---|---|
| Translate | `translate` | `[0.0, 0.0]` |  |
| Rotate | `rotate` | `0` | -180 … 180 |
| Scale | `scale` | `[1.0, 1.0]` |  |
| Extend | `extend` | `zero` | `zero` · `hold` · `repeat` · `mirror` |

### Under — `underTOP`

Input 1 over input 2 — Over, the other way up.

**Inputs:** in1, in2

| Parameter | Name | Default | Range |
|---|---|---|---|
| Opacity | `opacity` | `1` | 0 … 1 |

### Video Device In — `videodeviceinTOP`

Frames from a camera or capture device.

A camera, through ffmpeg. `Requested W`/`H` and the frame rate are requests rather than commands — a capture device only does the modes it does — so they are negotiated to the nearest mode the device actually reports, and the node shows what the device said if none will work.

On macOS the first use raises the system camera-permission prompt, and nothing arrives until it is granted. That prompt only appears for a properly signed bundle: macOS reads the usage description out of a *sealed* `Info.plist`, and a build whose bundle was never codesigned has none to read, so it is refused without ever being asked about. A refused camera is silent — the session opens, no error is printed and no frame ever comes — so the node says so itself after a few seconds rather than staying black with nothing to go on. The picture is always the newest frame decoded, so latency stays at about one frame; pausing the timeline pauses this like everything else in the network.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Device (blank = default) | `device` | *(empty)* |  |
| Requested W | `resw` | `1280` | 1 … 16384 |
| Requested H | `resh` | `720` | 1 … 16384 |
| Requested Frame Rate | `fps` | `30` | 1 … 240 |
| Active | `active` | `true` |  |

### Voronoi — `voronoiTOP`

Cellular noise, as flat cells, edges or a distance field.

Every pixel finds the nearest of a set of points scattered one per cell. What you do with that answer is `output`: `cells` flat-fills each region, which is stained glass; `edges` draws where two regions meet, which is cracked glass and antialiases for free because it is the *difference* between the two nearest distances; `distance` is the raw field, and is what you want when this feeds a Displace TOP rather than being looked at.

A generator, so nothing is wired in. Jitter at 0 is a regular grid and at 1 the points wander on their own phases, so the pattern boils rather than sliding as one sheet.

*No inputs — this is a generator.*

*Time dependent: cooks every frame, and everything downstream of it does too.*

| Parameter | Name | Default | Range |
|---|---|---|---|
| Cells | `scale` | `8` | 1 … 128 |
| Speed | `speed` | `0.4` | 0 … 8 |
| Jitter | `jitter` | `1` | 0 … 1 |
| Output | `output` | `cells` | `cells` · `edges` · `distance` |
| Color 1 | `color1` | `[0.0, 0.0, 0.0, 1.0]` |  |
| Color 2 | `color2` | `[1.0, 1.0, 1.0, 1.0]` |  |
| Resolution W | `resw` | `1280` | 1 … 4096 |
| Resolution H | `resh` | `720` | 1 … 4096 |

