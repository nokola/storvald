# Ergonomics

The pleasant, easy way wins: for the person who uses the app, the author
who writes a `.gogo` page, the programmer who calls an API, and the agent
that runs a command.

## What it means here

- Easy to understand, easy to write, easy to find.
- The easy way is the right way. If the right way takes more steps than
  the wrong one, the design is not finished.
- One keyword per concept, one way per task. Shorthand only as defined
  sugar. Nothing happens that the text does not show.
- A good default out of the box. A row shows its selection before anyone
  styles it.
- Quality, performance and a pleasant result are the goal. Matching an
  older app or an earlier version is a starting point, never the goal.

## The user decides the design

- Run a design change by the user BEFORE you build it: the keywords, what
  a property means, a default, the wording of an error that teaches, the
  shape of an API. Present the design and the trade-offs in plain prose,
  with a recommendation. Then wait.
- A question in reply to a proposal is not a go-ahead. Stay in discussion
  until an explicit "go".
- A mechanical fix inside settled rules needs no round.
- Scope is never the blocker: anything goes for UI and design changes.

## Raise a design question when you find it

When work uncovers a design question (what a framework should do, the
shape of an API, a class of mistakes with a structural fix), stop and
present it then: what surfaced, the options, the cost of each, a
recommendation. Do not take the local workaround and save the proposal
for the final report.

## Call out every difference

When this code differs from a reference (another app, an earlier
version), say so: what each does, and what the user sees in each. Add the
more pleasant alternative when there is one, and say which you recommend.
A difference no user can see gets one line.

## Say what you think

The user should not have to ask for your opinion. Weigh every request.
If it is not a good idea, say so before you act, with a better option
when you have one. This holds for a request to revert or to simplify too:
if your first approach was better, say why.
