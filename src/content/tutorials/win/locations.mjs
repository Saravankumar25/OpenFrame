export default {
  "id": "locations",
  "available": true,
  "reason": "",
  "path": "prep",
  "order": 3,
  "feature": "cast-crew-locations",
  "title": "Keep scouting notes on a location",
  "purpose": "Open a location and fill in its practical notes: parking, noise, permission, access and power.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open.",
    "At least one location is listed in Production > Locations."
  ],
  "objectives": [
    "Open a location from its card",
    "Scroll to the practical notes",
    "Record what the crew needs to know"
  ],
  "steps": [
    {
      "screen": "458-prod-locations",
      "action": "rightclick",
      "target": "role=button[name^=\"Rayapuram bus stand\"]",
      "label": "Right-click a location card",
      "text": null,
      "key": null,
      "body": "In Production, open Locations. It asks \"Which places can we use?\" Each card shows a name, a status and a scene count. Right-click a card."
    },
    {
      "screen": "461-prod-location-row-menu",
      "action": "click",
      "target": "role=menuitem[name=\"Open\"]",
      "label": "Choose Open",
      "text": null,
      "key": null,
      "body": "The menu offers Open, Archive and Delete. Choose Open."
    },
    {
      "screen": "462-prod-location-drawer",
      "action": "scroll",
      "target": "aside.drawer .db",
      "label": "Scroll down to the practical notes",
      "text": null,
      "key": null,
      "body": "The LOCATION drawer opens with the status switch (Idea, Shortlisted, Confirmed, Rejected), photos, Name *, address, contact and safety details. Scroll down to the practical notes."
    },
    {
      "screen": "463-prod-location-drawer-notes",
      "action": "view",
      "target": null,
      "label": "Fill in parking, noise, permission, access, power and other notes",
      "text": null,
      "key": null,
      "body": "PRACTICAL NOTES has fields for Parking, Noise, Permission, Access, Power, Toilets, Nearby facilities, Travel and Other notes. Fill in what you know. The nearest hospital and emergency contact are under SAFETY."
    }
  ],
  "result": "A location with its status and practical notes recorded, linked to the scenes that use it. A status change never alters the screenplay.",
  "mistakes": [
    "Thinking the status changes the script. Idea, Shortlisted, Confirmed and Rejected are for planning only.",
    "Skipping the safety fields. Nearest hospital and emergency contact are in the drawer for a reason.",
    "Deleting a location you only want hidden. Archive keeps it and Show archived brings it back."
  ],
  "troubleshooting": [
    [
      "I cannot find a location.",
      "Use Search locations or the status filter All / Idea / Shortlisted / Confirmed / Rejected."
    ],
    [
      "The practical notes are out of view.",
      "Scroll down inside the drawer."
    ],
    [
      "Add Photos opens a file picker.",
      "Choose pictures in the Windows picker. The first photo becomes the thumbnail and photos are stored in the project."
    ]
  ],
  "related": [
    "cast-crew",
    "schedule",
    "call-sheet",
    "breakdown"
  ],
  "next": "schedule"
};
