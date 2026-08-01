# Frontend Workflow & Acceptance Criteria

When making modifications to the frontend UI of the Familiar project, all AI coding assistants must adhere to the following workflow to ensure visual correctness and quality.

## Workflow Steps

1. **Implement Changes**: Make the requested HTML, CSS, or JS changes.
2. **Start Dev Server**: Start the local development server (e.g., `npm run tauri dev` or `cargo tauri dev`).
3. **Capture Visual State**: Use the `run_command` tool to execute `screencapture -m -t png tmp/ui_verification.png` (or similar) to capture the actual desktop state.
4. **Self-Review**: Use the `view_file` tool to inspect the screenshot. Evaluate it against the acceptance criteria below.
5. **Iterate**: If the UI does not meet the acceptance criteria (e.g., overlaps, misalignments, wrong sizing), adjust the code and repeat steps 2-4.
6. **Deliver**: Present the final screenshot to the user alongside the completed work. Do not claim the task is complete until the visual verification passes.

## Acceptance Criteria

- **No Unintended Overlapping**: Independent UI elements (e.g., the Pet, Bubble, and Stats Panel) must NOT overlap unless explicitly designed to do so. They should stack cleanly with appropriate spacing.
- **Alignment**: Elements must be properly aligned and anchored to their designated coordinates (e.g., Bubble anchored directly above the pet's head, Stats anchored directly below the pet's feet).
  - *Bubble Window Width*: The bubble window must precisely match the width of the main pet window (e.g., 320 logical pixels), with the bubble content centered inside it, ensuring they align perfectly as a unified column.
- **Responsiveness**: Text bubbles and panels must properly size themselves to their content (`fit-content`) without cutting off text or rendering with excessive empty space.
- **Transparency**: Borderless windows must maintain their transparent backgrounds without rendering unintended white or black boxes around the content.
- **Theming**: UI must match the defined aesthetic (e.g., dark mode, green accents, retro pixel art for the pet).
