export default {
  "id": "locations",
  "available": true,
  "reason": "",
  "path": "prep",
  "order": 3,
  "feature": "cast-crew-locations",
  "title": "Add a location and set its status",
  "purpose": "Add a place to your scouting notebook and mark it Confirmed.",
  "difficulty": "Beginner",
  "minutes": 5,
  "prerequisites": [
    "A project is open."
  ],
  "objectives": [
    "Add a location with a name",
    "Set its status",
    "Find photos, contact and safety details"
  ],
  "steps": [
    {
      "screen": "487-plan-location-add-empty",
      "action": "type",
      "target": "[data-id=\"nl-name\"]",
      "label": "In Production, Locations, click Add Location. Type the name, and the address and nearest hospital if you know them",
      "text": "Old Railway Station",
      "key": null,
      "body": "Open Production, then Locations, and click Add Location. In the Add Location sheet only the name is required. Type the name, and the address and nearest hospital if you know them."
    },
    {
      "screen": "488-plan-location-add-filled",
      "action": "click",
      "target": "[data-id=\"nl-submit\"]",
      "label": "Click Add Location",
      "text": null,
      "key": null,
      "body": "Pick a Status if you like, then click Add Location. A message confirms the new place."
    },
    {
      "screen": "489-plan-location-open",
      "action": "click",
      "target": "[data-id=\"loc-status\"] [aria-label=\"Confirmed\"]",
      "label": "The location opens in the inspector. Set its status to Confirmed",
      "text": null,
      "key": null,
      "body": "The location opens in the inspector. Set its status to Confirmed. The other choices are Idea, Shortlisted and Rejected."
    },
    {
      "screen": "490-plan-location-confirmed",
      "action": "view",
      "target": null,
      "label": "The card now reads Confirmed. Photos, contact and safety details live in the same inspector",
      "text": null,
      "key": null,
      "body": "The card now reads Confirmed. A status change never alters the screenplay. Photos, contact and safety details live in the same inspector, and Open Scenes shows the scenes that use the place."
    }
  ],
  "result": "A location in your notebook with a Confirmed status.",
  "mistakes": [
    "Filling every field before saving. Only the name is required; add the rest later in the inspector.",
    "Hiding a rejected place by deleting it. Use Archive, or set Rejected, so the history stays."
  ],
  "troubleshooting": [
    [
      "I cannot see my location.",
      "Check the status filter above the list. It may be set to a status the place does not have."
    ],
    [
      "An archived place is missing.",
      "Turn on Show archived."
    ]
  ],
  "related": [
    "cast-crew",
    "schedule",
    "breakdown"
  ],
  "next": "schedule"
};
