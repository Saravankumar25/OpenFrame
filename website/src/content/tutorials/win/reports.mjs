export default {
  "id": "reports",
  "available": true,
  "reason": "",
  "path": "shoot",
  "order": 3,
  "feature": "reports-budget",
  "title": "Save a report snapshot and export it",
  "purpose": "Keep a copy of a production report and export it as PDF, CSV or Excel.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A project is open with a breakdown and schedule."
  ],
  "objectives": [
    "Open a report",
    "Save a snapshot",
    "Open the export dialog and choose a format"
  ],
  "steps": [
    {
      "screen": "612-prod-reports-scene",
      "action": "click",
      "target": "role=button[name=\"Save snapshot\"]",
      "label": "On a report, click Save snapshot",
      "text": null,
      "key": null,
      "body": "In Production, open Reports. The buttons are Scene Report, Location Report, Cast Scene Report, Prop Report, Schedule Report and Breakdown Completeness. With a report open, click Save snapshot."
    },
    {
      "screen": "620-prod-reports-saved-snapshot",
      "action": "click",
      "target": "role=button[name=\"Back to live reports\"]",
      "label": "Click Back to live reports",
      "text": null,
      "key": null,
      "body": "The snapshot is listed under SAVED SNAPSHOTS. It does not change when production data changes. Click Back to live reports."
    },
    {
      "screen": "621-prod-reports-live-with-snapshots",
      "action": "click",
      "target": "role=button[name=\"Export\"]",
      "label": "Click Export",
      "text": null,
      "key": null,
      "body": "The live report shows with your snapshots listed. Click Export."
    },
    {
      "screen": "622-prod-reports-export-dialog",
      "action": "view",
      "target": null,
      "label": "Pick PDF, CSV or Excel (XLSX) and click Export…",
      "text": null,
      "key": null,
      "body": "The Export dialog offers FORMAT: PDF printable, CSV spreadsheet-friendly text, or Excel (XLSX) one sheet per table. Pick one, check the suggested file name and click Export. The Windows Save dialog then opens."
    }
  ],
  "result": "A saved snapshot of the report and an export in the format you chose. Private notes are excluded and exporting does not change your project.",
  "mistakes": [
    "Treating a snapshot as live. It keeps the numbers from the moment you saved it.",
    "Expecting branding in CSV. CSV stays plain, while production PDFs carry a small \"Created using OpenFrame\" line.",
    "Exporting before filtering. Use \"Filter report rows\" first if you need fewer rows."
  ],
  "troubleshooting": [
    [
      "The report looks out of date.",
      "Click Back to live reports. Snapshots do not refresh."
    ],
    [
      "The Export dialog does not save a file yet.",
      "Click Export in the dialog, then choose the folder in the Windows Save dialog."
    ],
    [
      "A report is empty.",
      "Reports read breakdown and schedule data. Tag elements and schedule scenes first."
    ]
  ],
  "related": [
    "budget",
    "sides",
    "breakdown",
    "schedule"
  ],
  "next": "budget"
};
