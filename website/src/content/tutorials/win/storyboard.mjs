export default {
  "id": "storyboard",
  "available": true,
  "reason": "",
  "path": "visual",
  "order": 3,
  "feature": "shot-lists-storyboards",
  "title": "Draw a storyboard panel",
  "purpose": "Sketch a frame with the built-in pen and add it to a scene as a storyboard panel.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open with at least one scene.",
    "You are in Production."
  ],
  "objectives": [
    "Open the Storyboards tab",
    "Start a sketch panel",
    "Draw with the pen and save the panel"
  ],
  "steps": [
    {
      "screen": "502-prod-moodboard-empty-canvas",
      "action": "click",
      "target": "role=button[name=\"Storyboards\"]",
      "label": "In Production, open the Storyboards tab",
      "text": null,
      "key": null,
      "body": "In Production, click the Storyboards tab. It asks \"What will the audience see?\" and shows the panels for the scene you pick."
    },
    {
      "screen": "503-prod-storyboard-panels",
      "action": "click",
      "target": "#radix-_r_f_",
      "label": "Click Add Panel",
      "text": null,
      "key": null,
      "body": "Click Add Panel at the top right. A menu opens with Import Image, Draw / Sketch and Empty Panel."
    },
    {
      "screen": "511-prod-storyboard-add-panel-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Draw / Sketch\"]",
      "label": "Choose Draw / Sketch",
      "text": null,
      "key": null,
      "body": "Choose Draw / Sketch. The sketch dialog opens with Pen and Eraser, three colours (Black, Red, Blue), Undo and Clear."
    },
    {
      "screen": "512-prod-storyboard-sketch-dialog",
      "action": "drag",
      "target": "img[aria-label=\"Sketch area\"]",
      "label": "Sketch the frame with the pen",
      "text": null,
      "key": null,
      "body": "Drag with the mouse inside the sketch area to draw the frame. The dialog notes that this is basic sketching only. For polished art, import an image made elsewhere."
    },
    {
      "screen": "513-prod-storyboard-sketch-drawn",
      "action": "view",
      "target": null,
      "label": "Click Save Panel to add the sketch as a new panel",
      "text": null,
      "key": null,
      "body": "When the drawing looks right, click Save Panel to add the sketch as a new panel. Click a panel to open its drawer for description, framing, camera movement, angle and duration."
    }
  ],
  "result": "A hand-sketched panel in the scene storyboard. You can reorder panels by dragging and export a panel sheet as PDF.",
  "mistakes": [
    "Trying to make finished art in the sketch tool. Use Import Image for artwork made elsewhere.",
    "Pressing Clear when you meant Undo. Clear wipes the whole sketch.",
    "Looking for a pasted picture. Press Ctrl+V on the Storyboards tab to add an image from the clipboard as a panel."
  ],
  "troubleshooting": [
    [
      "The sketch will not draw.",
      "Click and drag inside the sketch area itself, and check that Pen, not Eraser, is selected."
    ],
    [
      "Import Image opens a file picker.",
      "That is the Windows file picker. Choose a picture and it becomes a panel."
    ],
    [
      "I want a panel with no picture yet.",
      "Use Add Panel then Empty Panel, and add the visual later from the panel drawer."
    ]
  ],
  "related": [
    "shot-list",
    "moodboard",
    "write-screenplay",
    "breakdown"
  ],
  "next": "breakdown"
};
