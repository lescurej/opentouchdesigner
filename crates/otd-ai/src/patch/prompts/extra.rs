/// The extra brief for working from a reference image, appended to the user
/// turn when there is one.
///
/// It is separate from [`system_prompt`] on purpose. The system prompt says
/// what a patch is and never changes; this says what to do with the picture
/// sitting immediately above it, and putting it next to the image is what
/// keeps "reproduce this" attached to the thing being reproduced.
///
/// The whole content of it is: look before you build, and build the *recipe*
/// rather than the picture. A model asked to match an image reaches for one
/// enormous glslTOP that hard-codes what it sees, which is a screenshot with
/// extra steps — it cannot be turned, it does not move, and it teaches the
/// person nothing about their own patch.
pub fn reverse_engineer_prompt() -> &'static str {
    r#"WORKING FROM THE REFERENCE IMAGE ABOVE

Build a patch that produces this look. Not a copy of this exact frame — the
recipe that would generate it, and frames either side of it.

Read it first, in `notes`, in one line: what is the base pattern (noise,
ramp, shape, feedback trail), what has been done to its contrast, what are
the two or three colours, and is there evidence of a feedback loop — smearing,
echoes, concentric repeats, anything that looks like the frame before it is
still visible underneath.

Then build that, out of operators:

- Match the STRUCTURE before the colour. Getting "radial, high contrast,
  trailing" right and the hue wrong is close. The reverse is not.
- Colour last, and with a rampTOP composited on `multiply` — pick the ramp
  ends off the image. Two colours in one node beats three numbers in three.
- If it smears, echoes or tunnels, that is a feedback loop, and it is the
  first thing to build, not a garnish at the end.
- Reach for a glslTOP for the base pattern when no operator chain gives it,
  and keep it to the pattern. A single shader that draws the whole image is
  the failure mode here: nothing downstream can adjust it and no parameter
  does anything.
- Leave it TURNABLE. Whoever asked for this wants to push it somewhere else
  next, so the interesting quantities belong in parameters on separate nodes,
  not baked into one shader's constants.

Where the image shows something these operators genuinely cannot do, get as
close as they do reach and say what you left out in `notes`. Do not invent an
operator to cover the gap."#
}

/// The brief for a frame of the live camera: the picture is the INPUT to
/// design for, not a look to rebuild — which is what the reference brief
/// would make of it, and how a camera ends up replaced by a noise generator
/// painted the colour of someone's wall.
pub fn camera_prompt() -> &'static str {
    r#"THE LIVE CAMERA, AS IT LOOKS RIGHT NOW

The image above is one frame from the camera that is already in this network
(a videodeviceinTOP, or whatever the viewer shows). It is the INPUT. Do not
rebuild this picture and never replace the camera with a generator.

First, in `notes`, one line on what you see: what is in frame (a face close
up, a person full length, hands, a room, an object), how it is lit (dark,
bright, backlit, even) and how busy the background is.

Then build ONE striking effect ON the camera that suits it:

- Wire the existing camera node in. Do not add a second camera. Insert the
  chain before the existing output null so the viewer shows the result.
- Make it OBVIOUS. The person judges it at a glance, often while standing
  still, so the look must read in a single frame: ASCII, halftone, a tracking
  HUD, a point cloud, neon edges, thermal, a kaleidoscope. A tint, a slight
  grade or faint trails over a still subject look like nothing happened.
- Then add life: something that answers movement (feedback trails, frame
  differencing, datamosh, a tracker) on top of the look, not instead of it.
- Suit the frame: a face close up takes outlines, glow, RGB split, ASCII,
  halftone or thermal well; a full body or a busy room reads better as
  motion trails, a point cloud or a HUD; a dark frame needs levelTOP
  brightness first or every effect will be black.
- Keep the subject recognisable and the result neither black nor white.
- Leave two controls worth turning and name them in `notes`.

This network has no segmentation, pose, hand or face tracking and no depth
estimation. Do not promise background removal or body tracking; brightness,
difference, colour key and optical flow are what there is."#
}

/// The brief for a look at what a patch actually produced, with what the
/// pixels were measured to be doing wrong. `faults` are the tags from
/// [`crate::vision::Measured::faults`].
pub fn result_prompt(faults: &[&str]) -> String {
    let mut why = String::new();
    for fault in faults {
        why.push_str(match *fault {
            "BLACK" => "\n- BLACK: almost nothing reaches the output. A shader that samples no input, a feedback target that does not exist, a levelTOP crushing the picture, or the camera not wired into the chain the viewer shows.",
            "BLOWN" => "\n- BLOWN: the output is nearly white. Usually a feedback loop adding without decay; close it with `over` at partial opacity or decay the history below 1.",
            "STILL" => "\n- STILL: the output does not move while the camera does. Something between them is holding a frame or ignoring its input.",
            "PASSTHROUGH" => "\n- PASSTHROUGH: the output is the camera, unchanged. The effect is not in the chain the viewer shows — usually built beside the output null rather than before it.",
            "SUBTLE" => "\n- SUBTLE: the output is barely different from the camera — at a glance nothing happened. Replace the weak treatment with a bold look from the known-good list, keeping what already works.",
            other => other,
        });
    }
    format!(
        r#"WHAT YOUR LAST PATCH ACTUALLY PRODUCED

The image above is the viewer after your last change, cooked on the live
camera for about a second. Measured on its pixels:{why}

Fix exactly this, by retuning or rewiring the operators you created. Do not
start over, do not add a second camera, and say in `notes` what was wrong."#
    )
}
