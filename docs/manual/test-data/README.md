# Test data for the Manual Testing Guide

These files go with `docs/manual/OpenFrame_Studio_Manual_Testing_Guide.docx`. They are fictional and were written for OpenFrame manual testing. Do not use real scripts.

| File | What it is | Expected result when you import it |
|---|---|---|
| `BLACK-RAIN-valid.fountain` | A valid Fountain script, 12 scenes, 4 speaking characters (ARJUN, RAVI, MAYA, MEERA). | The import preview lists 12 scenes. The title BLACK RAIN is read from the title page. No blocking warning. |
| `BLACK-RAIN-valid.fdx` | The same story as a Final Draft file. | The same 12 scenes and 4 characters. |
| `special-characters.fountain` | Telugu, Hindi and Tamil text, quotes, ampersands, angle brackets, percent signs, a tab. | Text is kept exactly as typed. Nothing is dropped or turned into question marks. |
| `empty.fountain` | A file of 0 bytes. | Refused. The message is "This file is empty. Your current project was not changed." |
| `corrupt-script.fdx` | A Final Draft file cut off in the middle. | Refused on the failure screen. The message is "This Final Draft file is damaged or incomplete, so it could not be read. Your current project was not changed." No scene is added. |
| `wrong-type.pdf` | Plain text with a .pdf name. | Refused. The message is "This PDF could not be read. It may be damaged or password-protected. Your current project was not changed." No scene is added. |
| `make-oversized.ps1` | Creates `oversized-5MiB-plus-1.txt` (paste-limit test) and `oversized-100MiB-plus-1.fountain` (file-limit test). | The 5 MiB text pasted into the paste box is refused with "That is too much text to paste at once. Save it as a file and import the file instead." The 100 MiB file is refused by the file size limit. Nothing is added to the project. |

Built-in sample scripts that ship with the app are in `templates/samples/`: Example Feature (36 scenes), Example Short (10 scenes), Example Scene (1 scene, every element) and FDX Interchange Test (6 scenes).
