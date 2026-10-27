# Pailune UX principles

The design rules behind Pailune's interface (board item B-04), each tied to published evidence from
computer science (HCI), cognitive psychology, pedagogy and behavioural economics. Every rule has
an ID (`R-xx`) so code reviews and issues can refer to it.

Pailune's audience is gamers modding a single game on Linux. Most of them are not experts in
archives, symlinks or Proton, they want to play soon, and they care a great deal about not
breaking their game or their saves. The rules below are chosen for that situation.

> **Evidence strength.** Where findings are contested or come from industry reports rather than
> peer-reviewed studies, this is stated. Rules built on weak evidence are kept only when they are
> also cheap and harmless.

---

## 1. Make the system state visible

**Evidence.** Nielsen's heuristic #1, *visibility of system status* (Nielsen, 1994). Norman's
*gulf of evaluation*: people need to perceive and interpret the result of their action
(Norman, 2013). Response-time limits of 0.1 s, 1 s and 10 s for feeling in control, keeping the
flow of thought, and keeping attention (Nielsen, 1993, after Miller, 1968). An IBM industry
report argued that productivity rises sharply below roughly 400 ms system response (Doherty &
Thadani, 1982; not peer-reviewed).

- **R-01** The one thing that matters most — *does the game currently match my mod list?* — is
  always visible: a persistent banner says "Changes not applied" with the Apply button in it.
- **R-02** Every action gives feedback within 0.1 s (a pressed state, a status line); anything
  longer than ~1 s shows a job with a spinner; anything that can exceed 10 s shows progress.
- **R-03** Status text is specific and states the outcome ("Applied: 3 mods, 14 files"), not
  the operation ("Done").

## 2. Prevent errors, and make them recoverable

**Evidence.** Distinction between *slips* (right intention, wrong execution) and *mistakes*
(wrong intention) (Reason, 1990; Norman, 2013). Nielsen's heuristics #5 (error prevention) and #9
(help users recover). Shneiderman's golden rule *permit easy reversal of actions* (Shneiderman et
al., 2016). Confirmation dialogs suffer from habituation: people stop reading them, measurable even
in brain activity (Anderson et al., 2015), so a confirmation shown for every action protects
against nothing. Raskin (2000) argues for undo instead of confirmation.

- **R-04** Prefer **undo** over confirmation. Deleting a mod is undoable (trash + Undo), so it
  needs no dialog by default.
- **R-05** Reserve confirmation dialogs for actions that cannot be undone *and* are rare
  (Clean Slate, restoring a backup). Their text names the consequence, and the button names the
  action ("Restore backup", not "OK").
- **R-06** Keep destructive actions physically and visually apart from frequent ones, so a slip
  cannot hit them (no Clean Slate next to Apply).
- **R-07** Constrain input so invalid values cannot be entered (sliders and checkboxes instead of
  free text where possible) — a *forcing function* (Norman, 2013).
- **R-08** Error messages say what happened, why, and what to do next, in plain language
  (ISO 24495-1:2023).

## 3. Reduce cognitive load

**Evidence.** Working memory holds about four chunks (Cowan, 2001; earlier "seven ± two",
Miller, 1956). Cognitive load theory distinguishes intrinsic from extraneous load; good design
removes the extraneous part (Sweller, 1988; Sweller et al., 2019). Hick–Hyman law: decision time
grows with the logarithm of the number of equally likely choices (Hick, 1952; Hyman, 1953).
Recognition is easier than recall (Nielsen heuristic #6). Gestalt grouping by proximity and
similarity guides what belongs together (Wertheimer, 1923). Choice overload is often cited, but a
meta-analysis found an average effect near zero with large variation (Scheibehenne et al., 2010),
so Pailune does not hide options for that reason alone.

- **R-09** Progressive disclosure: show the common task first, put rare or expert options one
  level deeper (Settings categories, "Advanced" sections, details panel).
- **R-10** One primary action per screen, visually distinct; secondary actions quieter.
- **R-11** Labels and icons together (recognition), consistent terms everywhere — see the
  glossary in the user guide.
- **R-12** Group related controls and separate groups by whitespace and headings, not boxes
  everywhere.

## 4. Fast to operate

**Evidence.** Fitts' law: the time to hit a target grows with distance and shrinks with target
size (Fitts, 1954; MacKenzie, 1992). The keystroke-level model predicts expert task time from
elementary operations (Card, Moran & Newell, 1980). Nielsen heuristic #7: accelerators for
experts that novices never need to see.

- **R-13** Frequent targets (Apply, Play, the enable checkbox) are large and near where the
  user is looking; rare ones may be small.
- **R-14** Keyboard shortcuts for frequent actions, shown in tooltips and the guide.
- **R-15** Batch actions (enable all, multi-import, drag-and-drop) instead of repetition.

## 5. Learning by doing (onboarding)

**Evidence.** Minimalist instruction: users learn best from short, task-oriented material that
lets them act immediately and recover from errors; they skip long manuals (the *paradox of the
active user*, Carroll & Rosson, 1987; Carroll, 1990). Multimedia learning principles —
*segmenting* (small steps), *signaling* (highlight what matters), *pre-training* (key concepts
first) (Mayer, 2009). Worked examples reduce load for novices (Sweller & Cooper, 1985). Progress
cues raise completion: effort increases near a goal (goal-gradient; Kivetz, Urminsky & Zheng,
2006), and a head start increases completion (endowed progress; Nunes & Drèze, 2006).
Self-determination theory: competence and autonomy sustain motivation (Ryan & Deci, 2000).

- **R-16** First run is a short guided path of a few steps, each one a real action (find the game →
  import a mod → apply → play), never a slideshow to click through. Every step can be skipped
  (autonomy), and the guide can be reopened at any time.
- **R-17** Show progress through the steps; count the game detection as already done (an honest
  head start).
- **R-18** Teach the three core concepts first, in one sentence each: *mods are kept separately*,
  *Apply links them into the game*, *Unlink/Clean Slate undo it*.
- **R-19** Empty states teach: an empty list says what to do next and offers the button for it.
- **R-20** The built-in guide is task-oriented ("How do I…"), searchable, and short.

## 6. Defaults and framing

**Evidence.** People tend to keep defaults (status-quo bias; Samuelson & Zeckhauser, 1988); the
default option strongly shapes outcomes (Johnson & Goldstein, 2003; Thaler & Sunstein, 2008).
Losses weigh more than equal gains (prospect theory; Kahneman & Tversky, 1979), so worry about
"breaking the game" drives behaviour in a mod manager. The end of an experience weighs heavily in
its memory (peak-end rule; Kahneman et al., 1993).

- **R-21** Safe defaults: automatic daily backups on, undoable deletes, apply-before-play on,
  network checks off unless asked for.
- **R-22** Frame safety explicitly where users hesitate: "Nothing in the game folder is changed
  until you press Apply", "Unlink restores the original files".
- **R-23** End flows on a clear success state (e.g. after Apply: what changed, and "Play").

## 7. Accessible and adaptable

**Evidence.** WCAG 2.2: contrast of at least 4.5:1 for normal text, 3:1 for large text and UI
components; text resizable to 200 % (W3C, 2023). Motion can trigger vestibular symptoms; WCAG
2.3.3 (animation from interactions) and the `prefers-reduced-motion` practice. ISO 9241-110:2020
lists *individualisation* among its interaction principles.

- **R-24** Light and dark themes whose text colours meet 4.5:1 on their backgrounds.
- **R-25** UI scale adjustable (80–200 %) and a compact/comfortable density option.
- **R-26** A reduced-motion option disables animations.
- **R-27** Everything reachable by keyboard; focus is visible.

## 8. Settings

**Evidence.** Combination of the above: progressive disclosure (R-09), constraints (R-07),
recognition (R-11), and defaults (R-21). Users rarely change settings, so settings must be safe
when left alone and findable when needed.

- **R-28** Settings are grouped in a short list of categories; each setting has a one-line
  explanation next to it.
- **R-29** Appearance changes preview live; everything else is saved explicitly with
  *Save* / *Discard*, and each category can be reset to defaults.

---

## Where the rules live in Pailune

| Rules | Implementation |
|-------|----------------|
| R-01, R-10 | "Changes not applied" banner; Apply is the filled primary button only while changes are pending (`ui/views/header.rs`, `banner.rs`). |
| R-02, R-03 | Jobs panel with progress; status line states outcomes (`ui/views/jobs.rs`, `status_bar.rs`). |
| R-04, R-05 | Deletes go to the trash with Undo and skip the dialog unless *Ask before deleting* is on; confirmation buttons name the action (`ui/actions/mod.rs`, `ui/views/confirm.rs`). |
| R-06 | Clean slate sits in the *More* menu, away from Apply; Delete is set apart in the details panel. |
| R-07, R-28, R-29 | Settings use toggles, choices and bounded number fields, one-line help each, Save/Discard and per-page reset; appearance previews live (`ui/views/settings/`). |
| R-08 | Plain-language dialogs and errors (e.g. Clean slate, Nexus handler). |
| R-09, R-12 | Details panel, settings categories, Diagnostics page for technical state. |
| R-11 | One vocabulary: Enable/Disable, Apply/Unlink; glossary in the guide. |
| R-13 | Checkboxes enlarged (18 pt); primary buttons 30 pt high. |
| R-14 | Shortcuts listed in tooltips and the guide (F1, Ctrl+F, Ctrl+I, Ctrl+Enter, F5, Delete). |
| R-16 … R-20 | Guide window: three ideas, five real steps with progress and a head start, finish state, searchable topics; teaching empty states (`ui/guide/`, `ui/views/guide.rs`, `mod_list.rs`, `profiles.rs`). |
| R-21, R-22 | Safe defaults in `config/prefs.rs`; "Nothing in the game folder changes until you press Apply" where users start. |
| R-24 | Light and dark palettes; a unit test checks every text colour and button label for ≥ 4.5:1 (`ui/theme.rs`). |
| R-25, R-26 | UI size 80–200 %, compact/comfortable density, reduce motion; no text below 11.5 pt. |
| R-27 | egui keyboard focus (Tab) with visible focus rings. |

Checks that keep the rules true: `ui::theme` contrast test, `tests::glyphs` (every UI symbol has a
glyph in the bundled fonts), and `tests::screens` (renders every main screen in both themes for
review: `PYLUNE_SHOTS=<dir> cargo test screens -- --ignored`).

---

## References

- Anderson, B. B., Kirwan, C. B., Jenkins, J. L., Eargle, D., Howard, S., & Vance, A. (2015).
  How polymorphic warnings reduce habituation in the brain: Insights from an fMRI study.
  *Proceedings of CHI 2015*, 2883–2892.
- Card, S. K., Moran, T. P., & Newell, A. (1980). The keystroke-level model for user performance
  time with interactive systems. *Communications of the ACM*, 23(7), 396–410.
- Carroll, J. M. (1990). *The Nurnberg Funnel: Designing Minimalist Instruction for Practical
  Computer Skill.* MIT Press.
- Carroll, J. M., & Rosson, M. B. (1987). Paradox of the active user. In J. M. Carroll (Ed.),
  *Interfacing Thought*. MIT Press.
- Cowan, N. (2001). The magical number 4 in short-term memory. *Behavioral and Brain Sciences*,
  24(1), 87–114.
- Doherty, W. J., & Thadani, A. J. (1982). *The Economic Value of Rapid Response Time*
  (GE20-0752-0). IBM. (Industry report.)
- Fitts, P. M. (1954). The information capacity of the human motor system in controlling the
  amplitude of movement. *Journal of Experimental Psychology*, 47(6), 381–391.
- Hick, W. E. (1952). On the rate of gain of information. *Quarterly Journal of Experimental
  Psychology*, 4(1), 11–26.
- Hyman, R. (1953). Stimulus information as a determinant of reaction time. *Journal of
  Experimental Psychology*, 45(3), 188–196.
- ISO 9241-110:2020. *Ergonomics of human-system interaction — Part 110: Interaction principles.*
- ISO 24495-1:2023. *Plain language — Part 1: Governing principles and guidelines.*
- Johnson, E. J., & Goldstein, D. (2003). Do defaults save lives? *Science*, 302(5649), 1338–1339.
- Kahneman, D., Fredrickson, B. L., Schreiber, C. A., & Redelmeier, D. A. (1993). When more pain
  is preferred to less: Adding a better end. *Psychological Science*, 4(6), 401–405.
- Kahneman, D., & Tversky, A. (1979). Prospect theory: An analysis of decision under risk.
  *Econometrica*, 47(2), 263–291.
- Kivetz, R., Urminsky, O., & Zheng, Y. (2006). The goal-gradient hypothesis resurrected.
  *Journal of Marketing Research*, 43(1), 39–58.
- MacKenzie, I. S. (1992). Fitts' law as a research and design tool in human-computer
  interaction. *Human-Computer Interaction*, 7(1), 91–139.
- Mayer, R. E. (2009). *Multimedia Learning* (2nd ed.). Cambridge University Press.
- Miller, G. A. (1956). The magical number seven, plus or minus two. *Psychological Review*,
  63(2), 81–97.
- Miller, R. B. (1968). Response time in man-computer conversational transactions. *AFIPS Fall
  Joint Computer Conference*, 267–277.
- Nielsen, J. (1993). *Usability Engineering.* Academic Press.
- Nielsen, J. (1994). Enhancing the explanatory power of usability heuristics. *Proceedings of
  CHI 1994*, 152–158.
- Norman, D. A. (2013). *The Design of Everyday Things* (revised ed.). Basic Books.
- Nunes, J. C., & Drèze, X. (2006). The endowed progress effect. *Journal of Consumer Research*,
  32(4), 504–512.
- Raskin, J. (2000). *The Humane Interface.* Addison-Wesley.
- Reason, J. (1990). *Human Error.* Cambridge University Press.
- Ryan, R. M., & Deci, E. L. (2000). Self-determination theory and the facilitation of intrinsic
  motivation, social development, and well-being. *American Psychologist*, 55(1), 68–78.
- Samuelson, W., & Zeckhauser, R. (1988). Status quo bias in decision making. *Journal of Risk and
  Uncertainty*, 1(1), 7–59.
- Scheibehenne, B., Greifeneder, R., & Todd, P. M. (2010). Can there ever be too many options?
  A meta-analytic review of choice overload. *Journal of Consumer Research*, 37(3), 409–425.
- Shneiderman, B., Plaisant, C., Cohen, M., Jacobs, S., Elmqvist, N., & Diakopoulos, N. (2016).
  *Designing the User Interface* (6th ed.). Pearson.
- Sweller, J. (1988). Cognitive load during problem solving. *Cognitive Science*, 12(2), 257–285.
- Sweller, J., & Cooper, G. A. (1985). The use of worked examples as a substitute for problem
  solving in learning algebra. *Cognition and Instruction*, 2(1), 59–89.
- Sweller, J., van Merriënboer, J. J. G., & Paas, F. (2019). Cognitive architecture and
  instructional design: 20 years later. *Educational Psychology Review*, 31, 261–292.
- Thaler, R. H., & Sunstein, C. R. (2008). *Nudge.* Yale University Press.
- W3C (2023). *Web Content Accessibility Guidelines (WCAG) 2.2.* W3C Recommendation.
- Wertheimer, M. (1923). Untersuchungen zur Lehre von der Gestalt II. *Psychologische Forschung*,
  4, 301–350.
