Storvald overview:
- projects (can open side-by-side)
  - spans one or more folders
  - project memory - keep on task
    - the AI one
    - by the human - shows what's important to them
  - key info
    - per session - keep on task
      - ability to extract to project-level, with easy edit in-place
      - by AI
      - by human
    - per project
  - tasks/sessions
  - first-class support for todo tracking with my todo markdown formats
  - first-class support for decisions etc.
  - for decisions/todo tracking - sub areas etc etc.
  - ability to reorder sessions per project
- system-wide skills/tasks
  - guides through game and app development to avoid slop - e.g. will ask questions about app type etc etc
  - language guides - for comments, replies etc.
  - common mcps etc
  - support claude code
- explainer UI - "What did the AI do in this change?" with benefit language "speed up xyz" or "add abc"
  - benefits
  - risks if any
  - decisions
  - screenshots before/after, as applicable
- Running agents view
  - what's currently running
- other notes:
  - abilities (or just skills?)
  - auto-update 
    - for the skills/abilities
    - for the app itself
  - single portable .exe/.dmg etc file.
  - "Storvald is built in Storvald"

-
- A "waiting for you" list. Your attention is the bottleneck: commits to run, publishes, open calls like AD18 and AD21, across all projects.
- Evidence in the explainer. Each claim should link to the test, log line or measurement behind it, and list what was not checked. Without that, the explainer is the AI grading itself.
- Asked versus changed. Show what you asked for next to what was touched, as numbered items you can drop by number. That would have caught my README rewrite before you had to revert it.
- Rules as checks, not only guides. A guide is forgotten after a long session; a check that runs on every change is not (LF endings, no tracker ids in code).
- Permissions per project. What an agent may do alone, what needs your yes, and what is never allowed (push, publish, the live store id).
- Agents colliding, and jobs. Show which files each agent is touching, and show builds and test runs with their logs and counts.
- Task as the unit, not session. One task often spans several sessions, so "tasks/sessions" should not be one thing.

- Benefit language without proof. "Speeds up X" written by the agent that made the change is a sales pitch. Allow it only with a number attached, otherwise label it a claim.

- The anti-slop questionnaire. Questions about app type produce template answers, which is slop by another road. It only works if the guides carry your taste and examples.


-- System skills
-  Auto-update is the right way to deliver them. What I would still keep is small:
- A visible note of what changed. When my behaviour shifts between Monday and Tuesday, you should be able to see that a skill update is the cause.
- Updates take effect at the start of a session, not in the middle of one.
- One step back. If an update makes things worse, you can return to the previous version.
- System skills are read-only, with your own layer on top. If you could edit a system skill in place, the next update would overwrite your edit. Changes of yours belong in a project or user layer that the update never touches.

--
- A follow-up for each "yes". For example, "what goes in it?" or "which game does this the way you like?". That is where your taste enters.
- Ask when the thing is about to be built, not all upfront. Fifty questions on day one get clicked through, and the answers become slop themselves.
- Each question has a default and a "decide later".
- Answers land as decisions in the project. You already listed first-class decisions, so this fits.
- A list of what the AI decided without asking. You cannot ask everything. Showing the assumptions lets you overturn the ones you care about, and I think this does as much against slop as the questions do.

- The question lists per app type are also useful as a completeness check: pause, save, volume and controller remapping are the things people forget.