export default {
  "id": "screenplay-notes",
  "available": true,
  "reason": "",
  "path": "writing",
  "order": 5,
  "feature": "screenplay",
  "title": "Comment on the script and reply",
  "purpose": "Leave a comment on selected words and reply to it so the thread stays beside the script.",
  "difficulty": "Beginner",
  "minutes": 4,
  "prerequisites": [
    "A screenplay is open with some text.",
    "You have selected words and chosen Screenplay Tools > Add Comment (Option-Cmd-M)."
  ],
  "objectives": [
    "Write a comment on selected words",
    "Reply to it",
    "See the thread beside the script"
  ],
  "steps": [
    {
      "screen": "124-screenplay-comment-composer-quote",
      "action": "type",
      "target": "[data-id=\"comment-field\"]",
      "label": "Type your comment on the selected words, then click Comment",
      "text": "Should Kiran say which kite? The office has many.",
      "key": null,
      "body": "With words selected, choose Screenplay Tools > Add Comment. The inspector opens on the Comments tab with the quote. Type your comment in the Comment field, then click Comment."
    },
    {
      "screen": "125-screenplay-comment-posted",
      "action": "click",
      "target": "a[aria-label=\"Reply\"]",
      "label": "Click Reply under the comment",
      "text": null,
      "key": null,
      "body": "Your comment appears with its author and the quote. Click Reply under it."
    },
    {
      "screen": "126-screenplay-comment-reply-composer",
      "action": "type",
      "target": "textarea[aria-label=\"Reply\"]",
      "label": "Type a reply and click Reply",
      "text": "Yes, the red one with the newspaper tail.",
      "key": null,
      "body": "A Reply box opens. Type your reply and click Reply."
    },
    {
      "screen": "127-screenplay-comment-with-reply",
      "action": "view",
      "target": null,
      "label": "The comment and its reply stay beside the script",
      "text": null,
      "key": null,
      "body": "The comment and its reply stay beside the script. The toolbar button counts comments, for example Comments (1)."
    }
  ],
  "result": "Your script has a comment with a reply, kept beside the page.",
  "mistakes": [
    "Expecting private notes in this flow. This tutorial covers comments only.",
    "Forgetting to select words first. With nothing selected, Add Comment makes a scene comment instead.",
    "Leaving finished threads open. Click Resolve when a comment is dealt with."
  ],
  "troubleshooting": [
    [
      "I cannot find my comment.",
      "Open the Comments panel; it has Open and Resolved tabs."
    ],
    [
      "My comment is on the wrong words.",
      "Use Comment actions to remove it, then select the right words and add it again."
    ],
    [
      "Will comments print or export?",
      "Export says \"Private notes excluded.\" Check the Include notes option on the Export Screenplay sheet."
    ]
  ],
  "related": [
    "write-screenplay",
    "drafts-revisions",
    "export-script"
  ],
  "next": "drafts-revisions"
};
