# macOS development conventions

Always follow Apple's Human Interface Guidelines when developing the macOS app.
Use native macOS interaction patterns, controls, navigation, typography, spacing,
materials, and window and menu bar behavior. Support accessibility, keyboard
navigation, system appearance, and reduced motion where applicable.

Preserve the user's chosen colors unless they request a change. Adapt visual
references to native macOS conventions and the earbuds' supported capabilities.
Consult the relevant current Apple guidance when making unfamiliar design choices.

After every feature implementation, run checks appropriate to the change, fix
failures, then commit and push the changes to origin/main on
https://github.com/Pratikkk/QuickBuds.git. This is the user's standing authorization
for that workflow. Never force-push. Keep unrelated changes out of feature commits.
Report the checks run and any hardware behavior that remains unverified; do not
claim protocol tests validate physical Bluetooth lifecycle behavior.
