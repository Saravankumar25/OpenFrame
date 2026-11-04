export default {
  area: `Locations`,
  slug: `locations`,
  order: 11,
  part: `Plan`,
  intro: [
    `Locations is your scouting notebook. It answers one question: which places can we use? Each place you are considering for the film is a card with a status, a count of the scenes that use it, photos, an address, safety details, dates when it is free and practical notes such as parking, noise and power.`,
    `Every location has one of four statuses: Idea, Shortlisted, Confirmed or Rejected. The status is for planning. Changing it never alters the screenplay. When you reject a place that scenes still use, Straft offers to move those scenes to a practical replacement, again without editing the script.`,
    `Locations is the second Production tab after Catalog. It is on the Production page, in the tab strip under the project bar. Only a name is required to add a location, so you can start with a one-word idea and fill in details as you scout.`,
    `Everything you enter is stored in the project on this computer. Photos are stored in the project and work offline. Dates are advisory: the schedule warns about them and never moves a scene by itself.`,
  ],
  layout: [
    { name: `Header`, text: `The title Locations, the line Which places can we use? A practical scouting notebook., and the orange Add Location button at the top right.` },
    { name: `Toolbar`, text: `A Search locations box, then the status buttons All, Idea, Shortlisted, Confirmed and Rejected. At the far right is a Show archived check box.` },
    { name: `Location cards`, text: `A grid of cards, four across. Each card has a picture area (a map pin until you add a photo), the location name, a status label and the number of scenes, for example 1 scene or 0 scenes. The card you have open has an orange outline.` },
    { name: `Location drawer`, text: `A wide panel on the right, headed LOCATION and the name. From the top: the four status buttons, Photos, Name, Address / area, Contact, a SAFETY group (Nearest hospital, Emergency contact), Availability, a PRACTICAL NOTES group and the Scenes that use the place. The footer has Delete, Archive and Open Scenes.` },
  ],
  workflows: [
    {
      id: `locations-add`,
      title: `Add a location`,
      purpose: `Put a place you are considering into the notebook.`,
      level: `Beginner`,
      prerequisites: [`A project is open and your role can edit it.`],
      steps: [
        {
          screen: `prod-locations-empty`,
          do: `Click Production in the left rail, then Locations in the tab strip. On an empty page, click Add Location.`,
          explain: `A project with no locations says No locations yet. and Add a location you are considering for the film. Only the name is required. The orange Add Location button at the top right does the same.`,
        },
        {
          screen: `prod-location-add-dialog`,
          do: `In the Add Location dialog, type a Name *. Fill in Address / area, Contact and Nearest hospital if you know them, and choose a Status.`,
          explain: `The dialog says: Only the name is required. Confirming a location does not assign it to scenes. The Name box shows the example e.g. Old Railway Station. Status starts on Idea and can be Idea, Shortlisted, Confirmed or Rejected. Add Location stays greyed out until the name has text. Cancel closes the dialog without adding. You can fill in everything else later in the drawer.`,
        },
        {
          screen: `prod-locations`,
          do: `Click Add Location.`,
          explain: `The dialog closes, a message at the bottom says Added location with the name and an Undo link, and the new location opens in its drawer. Its card is in the grid with its status and 0 scenes.`,
        },
      ],
      result: `The location is on the page with the status you chose. Scenes are linked to it later from the Breakdown.`,
      cautions: [
        `Adding or confirming a location does not put it in any scene. To link scenes, tag the place as Location / Set in the Breakdown.`,
        `Contact is private. See Fill in scouting details.`,
      ],
      related: [`locations-fill-details`, `locations-set-status`],
    },
    {
      id: `locations-find`,
      title: `Search and filter locations`,
      purpose: `Show only the places that match a name or a status.`,
      level: `Beginner`,
      prerequisites: [`The page has at least one location.`],
      steps: [
        {
          screen: `prod-locations`,
          do: `Look at the cards. Type in the Search locations box.`,
          explain: `The picture shows four locations: Old cinema hall (Rejected, 1 scene), Pedda Revu tower (Idea, 0 scenes), Rayapuram bus stand (Confirmed, 1 scene) and Venkat Radio Works (Shortlisted, 0 scenes). The cards narrow as you type.`,
        },
        {
          screen: `prod-locations-filter-confirmed`,
          do: `Click Confirmed, or any other status button.`,
          explain: `The selected button turns dark. Here only Rayapuram bus stand remains. All shows every status again.`,
        },
        {
          screen: `prod-locations-no-match`,
          do: `If nothing matches, the page shows a short message.`,
          explain: `It says No locations match. In the picture the search box holds zzzz. Clear the box and click All to see every place.`,
        },
        {
          screen: `prod-locations`,
          do: `Tick Show archived at the top right to include archived locations.`,
          explain: `Archived locations are hidden by default. When shown they are paler and marked Archived.`,
        },
      ],
      result: `The grid shows only the locations you asked for.`,
      cautions: [
        `If a place seems missing, check the selected status button, the search box and Show archived.`,
      ],
      related: [`locations-archive`],
    },
    {
      id: `locations-set-status`,
      title: `Open a location and set its status`,
      purpose: `Move a place from Idea to Shortlisted to Confirmed, or mark it Rejected.`,
      level: `Beginner`,
      prerequisites: [`The page has at least one location.`],
      steps: [
        {
          screen: `prod-location-row-menu`,
          do: `Click a card to open it. You can also right-click the card and choose Open.`,
          explain: `The card menu has Open, Archive and a red Delete…. There is no separate Open Scenes in this menu, because the drawer has that button.`,
        },
        {
          screen: `prod-location-drawer`,
          do: `At the top of the drawer, click Idea, Shortlisted, Confirmed or Rejected.`,
          explain: `The selected status is dark. In the picture Rayapuram bus stand is Confirmed. The change saves at once and the card's label changes. Under the buttons a note says: A status change never alters the screenplay.`,
        },
        {
          screen: `prod-location-drawer-hospital`,
          do: `Click another card, for example Pedda Revu tower, to move to it.`,
          explain: `The drawer follows the card you click. Pedda Revu tower is at Idea and has Open Scenes greyed out, because no scene uses it. Click the x at the top right of the drawer to close it.`,
        },
      ],
      result: `The location shows its new status on its card and in the drawer.`,
      cautions: [
        `The status is only for planning. It does not change scenes or the screenplay.`,
        `A Rejected location still used in scenes shows a yellow note. See Replace a rejected location.`,
      ],
      related: [`locations-replace-rejected`, `locations-fill-details`],
    },
    {
      id: `locations-fill-details`,
      title: `Fill in scouting details and safety information`,
      purpose: `Record the address, contact, nearest hospital, emergency contact and practical notes for a place.`,
      level: `Beginner`,
      prerequisites: [`A location is open in its drawer.`],
      steps: [
        {
          screen: `prod-location-drawer`,
          do: `Edit Name *, Address / area and Contact.`,
          explain: `Fields save when you click out of them or press Enter. Press Esc in a one-line field to put back what was there. Name cannot be left empty: the old name comes back. Contact has a note: Private: visible to Owners and Editors only. Never shared with Straft AI or put in exports.`,
        },
        {
          screen: `prod-location-drawer-hospital`,
          do: `Under SAFETY, fill in Nearest hospital and Emergency contact.`,
          explain: `Nearest hospital asks for the name, address and phone of the nearest emergency room. The picture shows District hospital, 14 km and Coast guard station for Pedda Revu tower. Emergency contact is private in the same way as Contact. Fill these in before you plan a remote shoot day.`,
        },
        {
          screen: `prod-location-drawer-notes`,
          do: `Scroll the drawer down to PRACTICAL NOTES and fill in what you know.`,
          explain: `The note lines are Parking, Noise, Permission, Access, Power, Toilets, Nearby facilities, Travel and Other notes. Each shows a hint, such as Where can the unit park? or Sockets, generator needed?. Use as many as you need and leave the rest empty.`,
        },
      ],
      result: `The location holds the details the crew needs to plan a visit or a shoot day.`,
      cautions: [
        `If your role cannot see contacts, the Contact and Emergency contact lines say Contact details are hidden for your role.`,
        `Contact and Emergency contact are never put in exports.`,
      ],
      related: [`locations-photos`, `locations-availability`],
    },
    {
      id: `locations-photos`,
      title: `Add, reorder and remove location photos`,
      purpose: `Keep scouting pictures with the location, and choose the one that appears on its card.`,
      level: `Beginner`,
      prerequisites: [`A location is open in its drawer.`],
      steps: [
        {
          screen: `prod-location-drawer`,
          do: `Under Photos, click Add Photos and choose one or more pictures in the Windows window that opens.`,
          explain: `A note says: The first photo is the thumbnail. Photos are stored in the project and work offline. You can pick several pictures at once.`,
        },
        {
          screen: `prod-location-drawer`,
          do: `Use the small buttons on a photo: Move earlier, Move later or Remove photo.`,
          explain: `Move earlier and Move later change the order. The first photo becomes the picture on the card. Remove photo deletes the picture from the location, and a message at the bottom says Photo removed, with an Undo link.`,
        },
      ],
      result: `The location card shows your first photo in place of the map pin.`,
      cautions: [
        `If a picture file can no longer be read, its tile says Unavailable. Remove it and add it again.`,
      ],
      related: [`locations-fill-details`],
    },
    {
      id: `locations-availability`,
      title: `Record when a location is available`,
      purpose: `Add dates when a place is free or is not, so the schedule can warn you.`,
      level: `Intermediate`,
      prerequisites: [`A location is open in its drawer.`],
      steps: [
        {
          screen: `prod-location-drawer`,
          do: `Under Availability, choose Not available or Available in the first menu.`,
          explain: `The line above reads No dates entered. until you add some. A note under the form says: Dates are advisory: the schedule warns about them and never moves a scene by itself.`,
        },
        {
          screen: `prod-location-drawer`,
          do: `Choose a First day and a Last day, and type an optional Note.`,
          explain: `The date boxes show dd-mm-yyyy. Last day can be left empty for a single day, and it cannot be before First day. Add Dates is greyed out until a First day is chosen.`,
        },
        {
          screen: `prod-location-drawer-notes`,
          do: `Click Add Dates.`,
          explain: `The range appears above the form with a red Not available or green Available label, the dates and your note. A message says Updated availability of the location, with an Undo link. Click the x beside a range to remove it.`,
        },
      ],
      result: `The location lists its dated windows, and the schedule can warn when a scene is placed on a day that clashes.`,
      cautions: [
        `Dates never move a scene. They only produce warnings.`,
      ],
      related: [`locations-fill-details`],
    },
    {
      id: `locations-replace-rejected`,
      title: `Replace a rejected location in the scenes that use it`,
      purpose: `Move scenes from a place you rejected to a practical replacement, without editing the script.`,
      level: `Intermediate`,
      prerequisites: [`A location is Rejected but still used in scenes, and another location exists that is not Rejected.`],
      steps: [
        {
          screen: `prod-location-rejected-banner`,
          do: `Open the Rejected location and read the yellow note, then click Replace…`,
          explain: `The note reads Rejected, but still used in 1 scene. with a Replace… button. It appears only while scenes in the Production Source still use the place.`,
        },
        {
          screen: `prod-location-replace-dialog`,
          do: `In the Replace a rejected location dialog, choose the Practical replacement and tick the scenes to change.`,
          explain: `The dialog says which place is rejected and how many scenes use it. The replacement menu lists your other locations with their status, for example Rayapuram bus stand (Confirmed), and starts on a Confirmed one when there is one. Every scene starts ticked. If there is no other location, a blue note says to add the replacement as a location first (Idea, Shortlisted or Confirmed).`,
        },
        {
          screen: `prod-location-replace-dialog`,
          do: `Click Apply Replacement.`,
          explain: `A small note says this updates production associations only, and that the screenplay text, which still names the old place, is not changed. Edit the script yourself if you want that. A message says the replacement was applied to that many scenes, with an Undo link. Cancel closes the dialog.`,
        },
        {
          screen: `prod-location-rejected-banner`,
          do: `Check the drawer.`,
          explain: `Once the scenes use the replacement, the old location no longer shows the yellow note, and its drawer lists Practical replacement with the new name.`,
        },
      ],
      result: `The chosen scenes use the replacement location in production. The screenplay text is unchanged.`,
      cautions: [
        `The scene headings in your screenplay still name the old place. Change them in the Screenplay if you want them to match.`,
        `Apply Replacement is greyed out when no scene is ticked or there is nothing to replace with.`,
      ],
      related: [`locations-set-status`, `locations-see-scenes`],
    },
    {
      id: `locations-see-scenes`,
      title: `See which scenes use a location`,
      purpose: `Jump from a place to the scenes shot there.`,
      level: `Beginner`,
      prerequisites: [`A location is used by at least one scene.`],
      steps: [
        {
          screen: `prod-location-drawer-notes`,
          do: `Scroll to the bottom of the drawer and read Scenes.`,
          explain: `Each scene is a small label such as Scene 1. If no scene uses the place, it says No scenes use this location yet. Tag it in the Breakdown (Location / Set). Click a label to open that scene in the Breakdown.`,
        },
        {
          screen: `prod-location-drawer`,
          do: `Click Open Scenes in the drawer footer.`,
          explain: `The Breakdown opens on the first scene that uses the place. Open Scenes is greyed out when no scene uses it, as it is for Pedda Revu tower.`,
        },
      ],
      result: `The Breakdown shows a scene that is shot at the location.`,
      cautions: [
        `A scene gets its location from the Breakdown, where you tag Location / Set. Adding a location here does not do it.`,
      ],
      related: [`locations-add`],
    },
    {
      id: `locations-archive`,
      title: `Archive or restore a location`,
      purpose: `Hide a place you will not use for now, and bring it back later.`,
      level: `Beginner`,
      prerequisites: [`The page has at least one location.`],
      steps: [
        {
          screen: `prod-location-row-menu`,
          do: `Right-click a card and choose Archive. You can also open the location and click Archive in the drawer footer.`,
          explain: `There is no confirmation question. A message at the bottom says Archived with the name, and it has an Undo link.`,
        },
        {
          screen: `prod-locations`,
          do: `Tick Show archived to see archived places. Right-click one and choose Restore from Archive, or open it and click Restore.`,
          explain: `Archived cards are paler and marked Archived. A message says the location was restored from the archive.`,
        },
      ],
      result: `The location is hidden from the normal grid but kept, and Show archived brings it back.`,
      cautions: [
        `Archive keeps everything: photos, notes and the links to scenes. Use it instead of Delete when you may come back to a place.`,
      ],
      related: [`locations-delete`, `locations-find`],
    },
    {
      id: `locations-delete`,
      title: `Delete a location`,
      purpose: `Remove a location and know what happens to the scenes that use it.`,
      level: `Intermediate`,
      prerequisites: [`You have considered Archive first.`],
      steps: [
        {
          screen: `prod-location-row-menu`,
          do: `Right-click a card and choose the red Delete…. You can also click Delete in the drawer footer.`,
          explain: `Nothing is deleted until you confirm.`,
        },
        {
          screen: `prod-location-delete-confirm`,
          do: `Read the Delete “Rayapuram bus stand”? dialog and click Delete.`,
          explain: `It says the location moves to Recently Deleted for 30 days with its photos and availability. The scenes that use it keep their breakdown, and the screenplay is not changed. The dialog lists those scenes under Used in and any linked record, such as the matching catalog item. Cancel keeps the location.`,
        },
      ],
      result: `The location leaves the page. A message says it was deleted and can be restored from Recently Deleted, with an Undo link.`,
      cautions: [
        `You have 30 days to restore it from Recently Deleted.`,
        `Archive is the better choice when scenes still use the place.`,
      ],
      related: [`locations-archive`],
    },
  ],
  screens: {
    'prod-locations-empty': `The Locations page of a project with none. It says No locations yet. and Add a location you are considering for the film. Only the name is required, with an Add Location button. Search locations, the status buttons and Show archived are above. Used in locations-add.`,
    'prod-locations': `The Locations page with four cards: Old cinema hall (Rejected, 1 scene), Pedda Revu tower (Idea, 0 scenes), Rayapuram bus stand (Confirmed, 1 scene) and Venkat Radio Works (Shortlisted, 0 scenes). Add Location is at the top right. Used in locations-add, locations-find and locations-archive.`,
    'prod-locations-filter-confirmed': `The Locations page filtered to Confirmed. Only Rayapuram bus stand is shown. Used in locations-find.`,
    'prod-locations-no-match': `The Locations page with zzzz in the search box and no card shown. The page says No locations match. Used in locations-find.`,
    'prod-location-row-menu': `The menu of a location card, opened by right-click: Open, Archive and a red Delete…. Used in locations-set-status, locations-archive and locations-delete.`,
    'prod-location-drawer': `The LOCATION drawer for Rayapuram bus stand, status Confirmed. It shows the four status buttons, Add Photos, Name *, Address / area, Contact, the SAFETY fields, Availability with its date form, and a PRACTICAL NOTES heading. The footer has Delete, Archive and Open Scenes. Used in locations-set-status, locations-fill-details, locations-photos, locations-availability and locations-see-scenes.`,
    'prod-location-drawer-notes': `The same drawer scrolled to the lower part: the end of the availability form, the PRACTICAL NOTES lines (Parking, Noise, Permission, Access, Power, Toilets, Nearby facilities, Travel, Other notes) and Scenes with Scene 1. Variant, part of locations-fill-details; also used in locations-availability and locations-see-scenes.`,
    'prod-location-delete-confirm': `The Delete “Rayapuram bus stand”? dialog. It says the location moves to Recently Deleted for 30 days with its photos and availability, lists Scene 1 and a linked catalog item, and offers Cancel and a red Delete. Used in locations-delete.`,
    'prod-location-drawer-hospital': `The drawer for Pedda Revu tower, status Idea, with Address / area North headland, Nearest hospital District hospital, 14 km and Emergency contact Coast guard station. Open Scenes is greyed out. Used in locations-set-status and locations-fill-details.`,
    'prod-location-rejected-banner': `The drawer for Old cinema hall, status Rejected. A yellow note says Rejected, but still used in 1 scene. and has a Replace… button. Used in locations-replace-rejected.`,
    'prod-location-replace-dialog': `The Replace a rejected location dialog. It says Old cinema hall is Rejected and used in 1 scene, offers a Practical replacement menu (Rayapuram bus stand (Confirmed)), a ticked scene line and a note that the screenplay text is not changed. Cancel and Apply Replacement are at the bottom. Used in locations-replace-rejected.`,
    'prod-location-add-dialog': `The Add Location dialog: Name * (empty, with the example Old Railway Station), Address / area, Contact, Nearest hospital and Status (Idea). Add Location is greyed out until a name is typed. Used in locations-add.`,
  },
  shortcuts: [
    [`Ctrl+K`, `Search this project (shown in the search box at the top of the window)`],
  ],
  limits: [
    `A location has four statuses only: Idea, Shortlisted, Confirmed and Rejected. They are for planning and never change the screenplay.`,
    `Adding or confirming a location does not link it to a scene. Scenes are linked from the Breakdown.`,
    `Availability dates are advisory. The schedule warns about them and never moves a scene.`,
    `Contact and Emergency contact are private to Owners and Editors and are never shared with Straft AI or put in exports.`,
    `Replacing a rejected location changes production links only. The screenplay text keeps the old place name.`,
    `There is no map. A location holds an address as text and your own photos.`,
  ],
  troubleshooting: [
    {
      problem: `Add Location is greyed out in the dialog.`,
      fix: `The name is empty. Type a name. It is the only required field.`,
    },
    {
      problem: `I cannot find a location I added.`,
      fix: `Check the search box and the status button. Click All, clear the search, and tick Show archived in case it was archived.`,
    },
    {
      problem: `Open Scenes is greyed out.`,
      fix: `No scene uses the location. Tag it as Location / Set in the Breakdown first.`,
    },
    {
      problem: `A yellow note says Rejected, but still used in 1 scene.`,
      fix: `Click Replace… and choose a practical replacement. Or change the status back if you rejected the place by mistake.`,
    },
    {
      problem: `The Replace dialog says to add a replacement first.`,
      fix: `There is no other location that is not Rejected. Add one with Add Location, then try again.`,
    },
    {
      problem: `Add Dates is greyed out.`,
      fix: `Choose a First day. The button turns on once it has a date.`,
    },
    {
      problem: `The Contact line says Contact details are hidden for your role.`,
      fix: `Only Owners and Editors can see contacts. Ask an Owner or Editor.`,
    },
    {
      problem: `The Add Location, Archive or Delete buttons are missing.`,
      fix: `They show only when your role can edit the project.`,
    },
  ],
};
