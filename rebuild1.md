# TTS app

## App & UI

### Main window

Where all the config options lives.

#### Voice Catalog

TTS app provide different models/voice options.
The catalog allow the user to browse by language each available.
Each model/voice is downloadable and show metadata such as
- quality score
- size
- memory
- consumption.
- speed

#### Installed voices

Once downloaded they show up as installed model/voice.

#### Queue list

Same as Queue overlay but in the main window, without frosted glass style obviously.

### Tray icon

When the main window is closed, it goes into tray mode.

### Queue Overlay

When the main window is closed, Speech and Text playing/in queue are displayed with all control given by the Queue system.

Speech item have control buttons:
- play/stop/restart
- auto-play
- remove/skip
- download
- send back to the top of the queue to be re-processed (Queue::TextMessage)

Text item are also displayed with control buttons:
- remove

All items can be reordered.

The overlay appears at the top right corner, with frosted glass style.
It is only shown when the Queue contains an item.

## Backend

### Queue

The queue stores items of two kind TextMessage and Speech.

The queue can
- receive text as input (Queue::TextMessage)
- replace a TextMessage into a Queue::Speech
- give next TextMessage
- give next Speech
- reorder items

#### Text Message
The system host a queuing system, that accept and stores asyncrhonously a text (Queue::TextMessage)

#### Transcriber

The queue consumer takes TextMessage one by one an transform it to Queue::Speech

It process messages one by one:
- check the lang
- get active model for that lang
- load it if not yet loaded
- use the model to process it

The speech is the audio + metadata (text, lang, model used, ...)

Then it put it back in the queue to replace the initial TextMessage as Speech

#### Speech

The system host a queuing system, that accept and stores asyncrhonously a Speech (Queue::Speech)

#### SpeechPlayer

The player consume message one by one and play the audio.

The player can be controlled:
- play/stop/restart
- auto-play
- re-order Speech (Queue::Speech)
- remove/skip
- download
- send back to Queue::TextMessage

## Models

### Voice Catalog

Build on top of an hardcoded list of available voice/models and their metadata.

Item data:
- type (Kokoro, Piper, ...)
- quality score
- size
- memory
- consumption.
- speed
- files link

Capabilities of the catalog:
- install (install both voice and model, some type of model needs to be installed only once for all voices)
- uninstall (uninstall the )
- list available
- list installed

### Voice Preferences

Among installed model some are marked as active for a language.

Capability:
- get preferred model for lang
- set preffered model for lang

### Installed voices

It's the pair of a Model and a Voice.

Capabilities:
- load
- unload
- speak (TTS)

#### Model

The actual model, give him text and voice file and it read it at loud (or actually produce an audio transcription).

- load
- unload
- speak (TTS)

A model is an entity that can have different impl, it abstract it, but under the hood Kokoro, Piper and others have their own module to implement this abstraction.

#### Voice

The voice file.

A Voice is an entity that can have different impl, it abstract it, but under the hood Kokoro, Piper and others have their own module to implement this abstraction.




