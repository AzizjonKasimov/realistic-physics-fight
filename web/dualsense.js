// A PS5 DualSense driven directly over WebHID, for the rumble that Chrome
// and Edge on Windows cannot give it through the Gamepad API: Chromium's
// raw input reader there rumbles only the DualShock 4 among Sony's
// controllers. The player picks the controller once in the browser's
// window for it, which the page may open only just after the player did
// something (the page opens it on the controller's first press, which
// Chrome and Edge count as such, or on a click of its Turn on rumble
// buttons: index.html, rumbleOffer). The browser remembers the choice, and
// later visits take the controller at once. From then on the page reads its
// buttons and sticks from its own reports and sends its rumble.
//
// The report layouts follow SDL's HIDAPI PS5 driver (libsdl-org/SDL,
// src/joystick/hidapi/SDL_hidapi_ps5.c, zlib licence), rebuilt here from
// its description rather than copied: the effects block of 47 bytes, the
// Bluetooth framing and its CRC-32, the improved rumble of firmware 2.24
// and newer, and the input reports over USB and Bluetooth.
//
// Over Bluetooth the controller sends short reports until it receives its
// first output report, then full ones until it is switched off ("a one-way
// ticket", as SDL puts it). Programs that read it as a plain Windows game
// controller (DirectInput, and Chrome's own Gamepad API reader) see no
// buttons from then on, until it is switched off and on again. So once the
// page has a DualSense, it reads it here and hides the Gamepad API's copy.
window.fightDualSense = (function () {
  "use strict";

  const SONY = 0x054c;
  // The DualSense and the DualSense Edge.
  const PRODUCTS = [0x0ce6, 0x0df2];
  const FILTERS = PRODUCTS.map(function (productId) { return { vendorId: SONY, productId: productId }; });
  // Slots for the game (src/pads.rs, Pad::slot) apart from the Gamepad
  // API's own numbers.
  const FIRST_SLOT = 16;
  // The reports, by id.
  const USB_INPUT = 0x01;
  const BLUETOOTH_INPUT = 0x31;
  const USB_EFFECTS = 0x02;
  const BLUETOOTH_EFFECTS = 0x31;
  const FIRMWARE_INFO = 0x20;

  const supported = Boolean(window.isSecureContext && navigator.hid);
  // Each DualSense the page has opened: its device, its slot, whether it is
  // on Bluetooth, whether its firmware rumbles the newer way, what it last
  // reported, and the rumble it is playing.
  const open = [];

  function isDualSense(device) {
    return device.vendorId === SONY && PRODUCTS.indexOf(device.productId) >= 0;
  }

  function hasOutputReport(device, id) {
    return (device.collections || []).some(function (collection) {
      return (collection.outputReports || []).some(function (report) { return report.reportId === id; });
    });
  }

  // The standard CRC-32 (IEEE 802.3) of `bytes` after `start`, a CRC so far.
  const CRC_TABLE = (function () {
    const table = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) {
        c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      }
      table[n] = c >>> 0;
    }
    return table;
  })();
  function crc32(bytes, start) {
    let crc = (start === undefined ? 0 : start) ^ 0xffffffff;
    for (let i = 0; i < bytes.length; i++) {
      crc = CRC_TABLE[(crc ^ bytes[i]) & 0xff] ^ (crc >>> 8);
    }
    return (crc ^ 0xffffffff) >>> 0;
  }

  // The effects block for the motors at `strong` and `weak`, from 0 to 1:
  // the low-frequency motor (left) and the high-frequency one (right).
  // Firmware 2.24 and newer rumble through a newer flag; older firmware
  // through the rumble emulation flag, at half the values. Audio haptics off
  // in both.
  function effects(pad, strong, weak) {
    const block = new Uint8Array(47);
    const left = Math.round(Math.max(0, Math.min(1, strong)) * 255);
    const right = Math.round(Math.max(0, Math.min(1, weak)) * 255);
    block[0] = 0x02;
    if (pad.improvedRumble) {
      block[38] = 0x04;
      block[2] = right;
      block[3] = left;
    } else {
      block[0] |= 0x01;
      block[2] = right >> 1;
      block[3] = left >> 1;
    }
    return block;
  }

  // The output report that sets the motors, as WebHID sends it: its id and
  // its bytes after the id. Over USB the effects block alone; over
  // Bluetooth a tag byte, a fixed 0x10, the block, and the CRC-32 of the
  // whole report seeded with the Bluetooth output header 0xA2, little-endian
  // in its last four bytes.
  function effectsReport(pad, strong, weak) {
    const block = effects(pad, strong, weak);
    if (!pad.bluetooth) {
      return { id: USB_EFFECTS, bytes: block };
    }
    const bytes = new Uint8Array(77);
    bytes[0] = 0x00;
    bytes[1] = 0x10;
    bytes.set(block, 2);
    const crc = crc32(bytes.subarray(0, 73), crc32([0xa2, BLUETOOTH_EFFECTS]));
    bytes[73] = crc & 0xff;
    bytes[74] = (crc >>> 8) & 0xff;
    bytes[75] = (crc >>> 16) & 0xff;
    bytes[76] = (crc >>> 24) & 0xff;
    return { id: BLUETOOTH_EFFECTS, bytes: bytes };
  }

  function send(pad, strong, weak) {
    const report = effectsReport(pad, strong, weak);
    pad.device.sendReport(report.id, report.bytes).catch(function () {
      // Gone, or the browser refused this report; the next one may go.
    });
  }

  // An input report into the controller's state, in the browser's standard
  // layout (src/pads.rs, standard): the full report over USB (id 1, 63
  // bytes) or Bluetooth (id 0x31), or Bluetooth's short one (id 1, 9
  // bytes). Sticks from 0 to 255 about 128, triggers from 0 to 255; the
  // D-pad a hat in the low half of the face buttons' byte (0 up, then
  // clockwise, 8 none), the face buttons in its high half (0x10 square,
  // 0x20 cross, 0x40 circle, 0x80 triangle).
  function readReport(pad, id, data) {
    let sticks;
    let buttonsAt;
    let triggersAt;
    if (id === BLUETOOTH_INPUT && data.byteLength >= 11) {
      sticks = 1;
      triggersAt = 5;
      buttonsAt = 8;
      pad.bluetooth = true;
    } else if (id === USB_INPUT && data.byteLength >= 63) {
      sticks = 0;
      triggersAt = 4;
      buttonsAt = 7;
    } else if (id === USB_INPUT && data.byteLength >= 9) {
      sticks = 0;
      triggersAt = 7;
      buttonsAt = 4;
      pad.bluetooth = true;
    } else {
      return;
    }
    const byte = function (at) { return data.getUint8(at); };
    const stick = function (at) { return Math.max(-1, Math.min(1, (byte(at) - 128) / 127)); };
    pad.axes = [stick(sticks), stick(sticks + 1), stick(sticks + 2), stick(sticks + 3)];
    pad.triggers = [byte(triggersAt) / 255, byte(triggersAt + 1) / 255];
    const face = byte(buttonsAt);
    const shoulders = byte(buttonsAt + 1);
    const middle = byte(buttonsAt + 2);
    const hat = face & 0x0f;
    const down = [
      face & 0x20, face & 0x40, face & 0x10, face & 0x80,
      shoulders & 0x01, shoulders & 0x02, shoulders & 0x04, shoulders & 0x08,
      shoulders & 0x10, shoulders & 0x20, shoulders & 0x40, shoulders & 0x80,
      hat === 7 || hat === 0 || hat === 1,
      hat >= 3 && hat <= 5,
      hat >= 5 && hat <= 7,
      hat >= 1 && hat <= 3,
      middle & 0x01, middle & 0x02
    ];
    pad.buttons = down.reduce(function (bits, on, index) { return on ? bits | (1 << index) : bits; }, 0) >>> 0;
    pad.reported = true;
  }

  function freeSlot() {
    for (let slot = FIRST_SLOT; ; slot++) {
      if (!open.some(function (pad) { return pad.slot === slot; })) {
        return slot;
      }
    }
  }

  // Opens a DualSense the browser lets the page use, reads its firmware's
  // version (feature report 0x20, little-endian at its bytes 44 and 45
  // counting the report id, as SDL reads it), and sends it a report with
  // the motors off, which on Bluetooth also switches it to full reports.
  async function take(device) {
    if (!isDualSense(device) || open.some(function (pad) { return pad.device === device; })) {
      return;
    }
    try {
      if (!device.opened) {
        await device.open();
      }
    } catch (error) {
      return;
    }
    const pad = {
      device: device,
      slot: freeSlot(),
      bluetooth: hasOutputReport(device, BLUETOOTH_EFFECTS) && !hasOutputReport(device, USB_EFFECTS),
      // SDL's default when it cannot tell: up-to-date firmware.
      improvedRumble: true,
      buttons: 0,
      axes: [0, 0, 0, 0],
      triggers: [0, 0],
      reported: false,
      rumbling: { strong: 0, weak: 0, until: 0 },
      stop: null
    };
    device.addEventListener("inputreport", function (event) {
      readReport(pad, event.reportId, event.data);
    });
    open.push(pad);
    try {
      const info = await device.receiveFeatureReport(FIRMWARE_INFO);
      // Chrome gives a feature report with its id first; allow for none.
      const at = info.getUint8(0) === FIRMWARE_INFO ? 44 : 43;
      if (info.byteLength >= at + 2) {
        const version = info.getUint8(at) | (info.getUint8(at + 1) << 8);
        pad.improvedRumble = version === 0 || version >= 0x0224 || device.productId === 0x0df2;
      }
    } catch (error) {
      // Keep the newer way.
    }
    send(pad, 0, 0);
  }

  function drop(device) {
    const at = open.findIndex(function (pad) { return pad.device === device; });
    if (at >= 0) {
      clearTimeout(open[at].stop);
      open.splice(at, 1);
    }
  }

  if (supported) {
    navigator.hid.addEventListener("connect", function (event) { take(event.device); });
    navigator.hid.addEventListener("disconnect", function (event) { drop(event.device); });
    // A DualSense allowed on an earlier visit is taken at once.
    navigator.hid.getDevices().then(function (devices) { devices.forEach(take); }).catch(function () {});
  }

  let asking = false;
  return {
    supported: supported,
    // Whether the browser's window for picking a controller is open.
    asking: function () { return asking; },
    // Opens the browser's window for picking a DualSense. The browser opens
    // it only just after the player did something: a click, a key, or in
    // Chrome and Edge a controller's press, which they count as one. Says
    // "on" once the page drives the DualSense picked, "none" if the window
    // was closed with none, and "failed" if it could not open, or the
    // controller could not be used.
    request: async function () {
      if (!supported || asking) {
        return "failed";
      }
      asking = true;
      try {
        const devices = await navigator.hid.requestDevice({ filters: FILTERS });
        await Promise.all(devices.map(take));
        if (devices.length === 0) {
          return "none";
        }
        return open.some(function (pad) { return devices.indexOf(pad.device) >= 0; }) ? "on" : "failed";
      } catch (error) {
        return "failed";
      } finally {
        asking = false;
      }
    },
    // The DualSenses the page has open and has heard from, for the game:
    // each its slot, its buttons as bits, its axes, and its triggers' pull.
    pads: function () {
      return open.filter(function (pad) { return pad.reported; });
    },
    // Whether the page has a DualSense open, so the Gamepad API's copies
    // are hidden.
    any: function () { return open.length > 0; },
    // Whether a name the Gamepad API gives belongs to a DualSense.
    isDualSenseName: function (id) {
      const name = String(id).toLowerCase();
      return /dualsense/.test(name) || (/054c/.test(name) && /0ce6|0df2/.test(name));
    },
    // Rumbles the DualSense in `slot`, both motors from 0 to 1, for `ms`
    // milliseconds: false if the page has no DualSense there. A rumble that
    // comes while a stronger one plays keeps the stronger's strength and
    // end.
    rumble: function (slot, strong, weak, ms) {
      const pad = open.find(function (each) { return each.slot === slot; });
      if (!pad) {
        return false;
      }
      const now = performance.now();
      const playing = pad.rumbling;
      if (now < playing.until) {
        strong = Math.max(strong, playing.strong);
        weak = Math.max(weak, playing.weak);
      }
      const until = Math.max(now + ms, now < playing.until ? playing.until : 0);
      pad.rumbling = { strong: strong, weak: weak, until: until };
      send(pad, strong, weak);
      clearTimeout(pad.stop);
      pad.stop = setTimeout(function () {
        pad.rumbling = { strong: 0, weak: 0, until: 0 };
        send(pad, 0, 0);
      }, until - now);
      return true;
    },
    // For checks: the report builders and the CRC-32.
    crc32: crc32,
    effectsReport: effectsReport,
    readReport: readReport
  };
})();
