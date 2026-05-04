# HID Usage Tables — Power & Battery System Pages

Extracted from *HID Usage Tables for Universal Serial Bus (USB)*, Version 1.3 (January 1, 2022).
© 1996–2022 USB Implementers' Forum. All rights reserved.

## About this document

The Human Interface Device (HID) class is a USB device class for human-operated
peripherals (keyboards, mice, game controllers, etc.) and, by extension, any
device whose data fits the HID report model. A HID device exposes its data
layout through a **Report Descriptor** built from items that reference
**Usages**.

A **Usage** is a 32-bit identifier composed of a 16-bit **Usage Page** and a
16-bit **Usage ID**. The Usage Page selects a category (e.g. Generic Desktop,
Keyboard, Sensors); the Usage ID selects a specific control or data field
within that page. The Usage tells the host *what* a value means — `Voltage`,
`Battery Present`, `Charging` — independently of *how* it is encoded in the
report.

This file contains only two Usage Pages from the spec:

- **Power Page (0x84)** — generic power-device modeling: UPS, power supplies,
  battery systems, chargers, outlets, flows, and power summaries, including
  measurements (voltage, current, frequency, …), configuration, controls
  (switch on/off, delays, test, alarm), and status flags.
- **Battery System Page (0x85)** — Smart Battery System reporting aligned with
  the SBS specification: battery mode, status, alarms, charger mode/status,
  selector state, and battery measures (state of charge, run time, cycle
  count, …).

### Usage Type abbreviations

The **Usage Types** column uses the standard HID type codes defined in §3.4 of
the spec:

**Data types**

| Abbr | Type | Meaning |
|---|---|---|
| **SV** | Static Value | Read-only multi-bit value (constant) |
| **SF** | Static Flag | Read-only single-bit flag declaring a fixed feature |
| **DV** | Dynamic Value | Read/write multi-bit value |
| **DF** | Dynamic Flag | Read/write single-bit flag |
| **Sel** | Selector | One element of a Named Array |

**Collections** (group other items)

| Abbr | Type | Meaning |
|---|---|---|
| **CA** | Collection Application | Top-level collection identifying the device class |
| **CL** | Collection Logical | Groups related items together |
| **CP** | Collection Physical | Groups items collected at the same physical point |
| **NAry** | Named Array | Collection wrapping a set of `Sel` selectors so one is active at a time |

A "/" between two types (e.g. `SV/DV`, `CL/CP`) means the same usage may be
declared as either, depending on whether the value is fixed or
runtime-controllable, or whether the collection groups logical or physical
items. Usage types are guidance — the actual interpretation also depends on
the Main item flags (Constant/Variable, Absolute/Relative, etc.) declared
with the field.

### Default units convention

Many usages declare a default physical unit (Volts, Amps, Hertz, seconds,
percent, …). The default may be overridden in the report descriptor with
explicit `Unit` and `Unit Exponent` items, except where a usage description
states the unit cannot be overridden.

---

## 29. Power Page (0x84)

| Usage ID | Usage Name | Usage Types | Section |
|---|---|---|---|
| 00 | *Undefined* |  |  |
| 01 | iName | SV | 29.4 |
| 02 | **Present Status** | CL | 29.4 |
| 03 | **Changed Status** | CL | 29.4 |
| 04 | **UPS** | CA | 29.4 |
| 05 | **Power Supply** | CA | 29.4 |
| 06–0F | *Reserved* |  |  |
| 10 | **Battery System** | CP | 29.4 |
| 11 | Battery System Id | SV | 29.4 |
| 12 | **Battery** | CP | 29.4 |
| 13 | Battery Id | SV | 29.4 |
| 14 | **Charger** | CP | 29.4 |
| 15 | Charger Id | SV | 29.4 |
| 16 | **Power Converter** | CP | 29.4 |
| 17 | Power Converter Id | SV | 29.4 |
| 18 | **Outlet System** | CP | 29.4 |
| 19 | Outlet System Id | SV | 29.4 |
| 1A | **Input** | CP | 29.4 |
| 1B | Input Id | SV | 29.4 |
| 1C | **Output** | CP | 29.4 |
| 1D | Output Id | SV | 29.4 |
| 1E | **Flow** | CP | 29.4 |
| 1F | Flow Id | SV | 29.4 |
| 20 | **Outlet** | CP | 29.4 |
| 21 | Outlet Id | SV | 29.4 |
| 22 | **Gang** | CL/CP | 29.4 |
| 23 | Gang Id | SV | 29.4 |
| 24 | **Power Summary** | CL/CP | 29.4 |
| 25 | Power Summary Id | SV | 29.4 |
| 26–2F | *Reserved* |  |  |
| 30 | Voltage | DV | 29.5 |
| 31 | Current | DV | 29.5 |
| 32 | Frequency | DV | 29.5 |
| 33 | Apparent Power | DV | 29.5 |
| 34 | Active Power | DV | 29.5 |
| 35 | Percent Load | DV | 29.5 |
| 36 | Temperature | DV | 29.5 |
| 37 | Humidity | DV | 29.5 |
| 38 | Bad Count | DV | 29.5 |
| 39–3F | *Reserved* |  |  |
| 40 | Config Voltage | SV/DV | 29.6 |
| 41 | Config Current | SV/DV | 29.6 |
| 42 | Config Frequency | SV/DV | 29.6 |
| 43 | Config Apparent Power | SV/DV | 29.6 |
| 44 | Config Active Power | SV/DV | 29.6 |
| 45 | Config Percent Load | SV/DV | 29.6 |
| 46 | Config Temperature | SV/DV | 29.6 |
| 47 | Config Humidity | SV/DV | 29.6 |
| 48–4F | *Reserved* |  |  |
| 50 | Switch On Control | DV | 29.7 |
| 51 | Switch Off Control | DV | 29.7 |
| 52 | Toggle Control | DV | 29.7 |
| 53 | Low Voltage Transfer | DV | 29.7 |
| 54 | High Voltage Transfer | DV | 29.7 |
| 55 | Delay Before Reboot | DV | 29.7 |
| 56 | Delay Before Startup | DV | 29.7 |
| 57 | Delay Before Shutdown | DV | 29.7 |
| 58 | Test | DV | 29.7 |
| 59 | Module Reset | DV | 29.7 |
| 5A | Audible Alarm Control | DV | 29.7 |
| 5B–5F | *Reserved* |  |  |
| 60 | Present | DF | 29.8 |
| 61 | Good | DF | 29.8 |
| 62 | Internal Failure | DF | 29.8 |
| 63 | Voltage Out Of Range | DF | 29.8 |
| 64 | Frequency Out Of Range | DF | 29.8 |
| 65 | Overload | DF | 29.8 |
| 66 | Over Charged | DF | 29.8 |
| 67 | Over Temperature | DF | 29.8 |
| 68 | Shutdown Requested | DF | 29.8 |
| 69 | Shutdown Imminent | DF | 29.8 |
| 6A | *Reserved* |  |  |
| 6B | Switch On/Off | DF | 29.8 |
| 6C | Switchable | DF | 29.8 |
| 6D | Used | DF | 29.8 |
| 6E | Boost | DF | 29.8 |
| 6F | Buck | DF | 29.8 |
| 70 | Initialized | DF | 29.8 |
| 71 | Tested | DF | 29.8 |
| 72 | Awaiting Power | DF | 29.8 |
| 73 | Communication Lost | DF | 29.8 |
| 74–FC | *Reserved* |  |  |
| FD | iManufacturer | SV | 29.9 |
| FE | iProduct | SV | 29.9 |
| FF | iSerialNumber | SV | 29.9 |
| 100–FFFF | *Reserved* |  |  |

*Table 29.1: Power Page*

### 29.1 Power and Battery Device Overview

A Power Device is a set of interconnected power modules (Battery Systems, Power Converters, Outlet Systems, and Power Summaries). Each module may include one or several interconnected sub-modules. Some sub-modules are located inside modules (Batteries, Chargers) and some are located at the interface of modules (Inputs, Outputs, and Outlets). All modules, sub-modules, and interconnections are defined as objects.

#### 29.1.1 Battery

A Battery is typically a sealed pack of rechargeable electrochemical cells that provides a primary or auxiliary source of stored direct current (DC) energy to electronic devices. Some examples are the battery pack for cellular phones (principal source), the battery pack(s) for notebook computers (auxiliary source), and the sealed batteries in uninterruptible power supplies (auxiliary source).

Battery management may differ significantly for different Power Devices. It is therefore necessary to define three battery models, see Battery Settings.

See Battery System Page (0x85) for Usages to comply with the Smart Battery Specification.

#### 29.1.2 Charger

A Charger is typically a controlled converter (AC/DC or DC/DC) that charges batteries.

#### 29.1.3 Input and Output

Inputs and Outputs are the connection points of a module with other modules. They are associated with dynamic data such as electric measurement and status. In addition to basic features such as Voltage, Current or Frequency, they may include controls such as 'Switch On Control' or 'Switch Off Control'.

#### 29.1.4 Battery System

A Battery System (see Battery System Page (0x85)) is a collection of Batteries, Charger, Inputs, and Outputs. Battery Systems have intelligent switching systems that provide a solution for many of the complexities associated with the implementation of multiple-battery systems such as notebook computers.

#### 29.1.5 Power Supply or Power Converter

A Power Supply or Power Converter is an electrical converter of source energy of a particular voltage, frequency, and current into a different specific voltage, frequency, and current. Typical supplies are AC to DC, DC to DC, DC to AC, AC to AC, and AC to DC to AC. Some examples are PC/notebook power supplies (AC to DC), battery chargers (AC to DC or DC to DC), and uninterruptible power supplies (AC to DC to AC). A Power Supply has Inputs and Outputs.

#### 29.1.6 Outlet and Outlet System (or Power Source Node)

In its most general sense, an Outlet System is a set of physical connections by which devices requiring electrical energy are attached to a power source. The attachment point may be switched (capable of on/off control) or unswitched (incapable of on/off control). Of interest to the Power Device are outlets that are capable of being remotely switched. Examples are certain rackmount/enclosure-outlet receptacle strips and some uninterruptible power supplies. An Outlet is an individual switch and an Outlet System is a set of Outlets.

#### 29.1.7 Gang

A Gang is a set of objects that have the same properties and act together. For example, a Gang of Outlets is composed of different Outlets that are connected to the same power source. If they are switchable, then they are switched by the same local or remote on/off control.

#### 29.1.8 Flow

The electric power Flows are an abstraction of power lines that power some Inputs (external to a module), are generated by some Outputs (a module to the external world), and may connect some Outputs to some Inputs (inter-module relation). Flow defines only the electric configuration of the power line.

#### 29.1.9 Power Summary

The Power Summary is an abstraction that summarizes data from the power source that supplies the load of the Power Device. Its configuration is defined by an associated Flow. There is associated dynamic data defining the present power source (AC Input, Battery, etc.) of the Flow. Implemented in a Power Device that includes a battery, the Power Summary contains the same information as ACPI Battery Control Methods.

All of the data of the power source that supplies a particular load of a Power Device is distributed through different related modules. Without a Power Summary, an application would have to browse all of these modules in order to get the pertinent data. The Power Summary module therefore facilitates power management application design.

Power Management software (e.g., Microsoft OnNow) could use a Power Summary to associate a USB Node with its power source. Implementing only a Power Summary within a Power Device is the simplest way to expose characteristics of a power source to power management.

### 29.2 Object Definitions and Properties

An object is composed of a set of the following data items or collections of these data items:

- **Controls**: Manipulate present state or setting of the object.
- **Settings**: Factory settings.
- **Status**: Present or Changed status.
- **Measures**: Values related to Electrical or Power Devices.

Each object has a unique identifier (ID). The ID identifies the object inside a type. It is included in the static data of each object and used to define links between objects.

The object hierarchy of a Power Device is the following:

1. Battery Systems (zero to many), each having:
   - Inputs (zero to many), each being connected to an input Flow.
   - Chargers (one to many).
   - Batteries (one to many), each capable of being exclusively connected to a Charger or to an Output.
   - Outputs (one to many), each being connected to an output Flow.
2. Power Converters (zero to many), each having:
   - Inputs (one to many), each being connected to an output Flow and capable of being connected to any Output.
   - Outputs (one to many), each being connected to an input Flow and capable of being connected to any Input.
3. Outlet Systems (zero to many), each having:
   - Individual Outlets (1 to many), each being connected to an output Flow.
   - One input Flow.
   - Output Flow (one per Outlet).
   - Power Summary (zero to many), each being connected to an output Flow.

The sub-modules of a module are directly connected. For example, an Input is connected to a Charger inside a Battery System, or an Input is connected to an Output inside a Power Converter.

The different modules are connected to each other and to entities outside the Power Device by Flows. The connection points are the Inputs and the Outputs of the modules. For example, a Flow connects the outside world to an Input of a Battery System; it is the main AC Flow. Or, a Flow connects the Output of a Battery System to the Input of a Power Converter; it is the battery backup DC Input of the Converter.

The connection inside or outside a module could be static or dynamically controlled. For example, the connection of an Input to a Charger inside a Battery System is generally static. Or, the connection of an Input to an Outlet inside an Outlet System is generally dynamically controlled.

### 29.3 Power Device Examples

Power Devices can be implemented with one or more objects. The examples below illustrate how multiple objects can be contained in a single device.

#### 29.3.1 A Simple Power Supply

This configuration contains the following objects:

- One DC Output Flow (optional)
- One Power Summary

#### 29.3.2 The Power Supply of a Typical USB Device

This configuration contains the following objects:

- One AC Input Flow, one DC Input Flow (USB Bus Power)
- One Power Converter consisting of one AC Input, one DC Input, and one DC Output
- One DC Output Flow
- One Power Summary

#### 29.3.3 A Rackmount Receptacle Strip with Three Outlets

This configuration contains the following objects:

- One AC Input Flow
- One Outlet System consisting of one AC Input and three individual AC Outlets
- Three AC Output Flows

#### 29.3.4 A Simple UPS with One Non-Switchable Output

This configuration contains the following objects:

- One AC Input Flow (Main AC)
- One Battery System consisting of one AC Input, one Battery, one Charger, and one DC Output
- One DC Flow (Backup DC)
- One Power Converter consisting of one DC Input, one AC Input and one AC Output
- One AC Output Flow (AC Flow)
- One Power Summary

#### 29.3.5 A UPS with One Non-Switchable Output and Two Switchable Outlets

This configuration contains the following objects:

- One AC Input Flow (Main AC)
- One Battery System consisting of one AC Input, one Battery, one Charger, and one DC Output
- One DC Flow (Backup DC)
- One Power Converter consisting of one DC Input, one AC Input, and one AC Output
- One AC Output Flow (AC Flow)
- One Outlet System with two outlets
- Two AC Output Flows (AC Flow)

### 29.4 Power Devices

| Usage Name | Usage Type | Description |
|---|---|---|
| iName | SV | Index of a string descriptor containing the physical description of the object |
| **Present Status** | CL | Present status information related to an object |
| **Changed Status** | CL |  |
| **UPS** | CA | Defines an Uninterruptible Power Supply |
| **Power Supply** | CA | Defines a Power Supply |
| **Battery System** | CP | Defines a Battery System power module |
| Battery System Id | SV | Indicates a particular Battery System |
| **Battery** | CP | Defines a Battery |
| Battery Id | SV | Indicates a particular Battery |
| **Charger** | CP | Defines a Charger |
| Charger Id | SV | Indicates a particular Charger |
| **Power Converter** | CP | Defines a Power Converter power module |
| Power Converter Id | SV | Indicates a particular Power Converter |
| **Outlet System** | CP | Defines an Outlet System power module |
| Outlet System Id | SV | Indicates a particular Outlet System |
| **Input** | CP | Defines an Input |
| Input Id | SV | Indicates a particular Input |
| **Output** | CP | Defines an Output |
| Output Id | SV | Indicates a particular Output |
| **Flow** | CP | Defines a Flow |
| Flow Id | SV | Indicates a particular Flow |
| **Outlet** | CP | Defines an Outlet |
| Outlet Id | SV | Indicates a particular Outlet |
| **Gang** | CL/CP | Defines a Gang |
| Gang Id | SV | Indicates a particular Gang |
| **Power Summary** | CL/CP | Defines a Power Summary |
| Power Summary Id | SV | Indicates a particular Power Summary |

### 29.5 Power Measures

| Usage Name | Usage Type | Description |
|---|---|---|
| Voltage | DV | Actual value of the voltage. (Default units in Volts) |
| Current | DV | Actual value of the current. (Default units in Amps) |
| Frequency | DV | Actual value of the frequency. (Default units in Hertz) |
| Apparent Power | DV | Actual value of the apparent power (Default units in Volt-Amps) |
| Active Power | DV | Actual value of the active (RMS) power (Default units in Watts) |
| Percent Load | DV | The actual value of the percentage of the power capacity presently being used on this input or output line, i.e., the greater of the percent load of true power capacity and the percent load of Apparent Power. |
| Temperature | DV | The actual value of the temperature. (Default units in degrees Kelvin) |
| Humidity | DV | The actual value of the humidity (Default unit is %) |
| Bad Count | DV | The number of times the device, module, or sub-module entered a bad condition (e.g., an AC Input entered an out-of-tolerance condition). |

### 29.6 Power Configuration

| Usage Name | Usage Type | Description |
|---|---|---|
| Config Voltage | SV/DV | Nominal value of the voltage. (Default units in Volts) |
| Config Current | SV/DV | Nominal value of the current. (Default units in Amps) |
| Config Frequency | SV/DV | Nominal value of the frequency. (Default units in Hertz) |
| Config Apparent Power | SV/DV | Nominal value of the apparent power (Default units in Volt-Amps) |
| Config Active Power | SV/DV | Nominal value of the active (RMS) power (Default units in Watts) |
| Config Percent Load | SV/DV | Nominal value of the percentage load that could be used without critical overload |
| Config Temperature | SV/DV | Nominal value of the temperature. (Default units in 0.1 degrees Kelvin) |
| Config Humidity | SV/DV | Nominal value of the humidity (Default unit is %) |

### 29.7 Power Control

| Usage Name | Usage Type | Description |
|---|---|---|
| Switch On Control | DV | Controls the Switch On sequence. **Write Value:** 0 = Stop Sequence, 1 = Start Sequence. **Read Value:** 0 = None, 1 = Started, 2 = In Progress, 3 = Completed |
| Switch Off Control | DV | Controls the Switch Off sequence. **Write Value:** 0 = Stop Sequence, 1 = Start Sequence. **Read Value:** 0 = None, 1 = Started, 2 = In Progress, 3 = Completed |
| Toggle Control | DV | Controls the Toggle sequence. A Toggle sequence is a Switch Off sequence followed immediately by a Switch On sequence. **Write Value:** 0 = Stop Sequence, 1 = Start Sequence. **Read Value:** 0 = None, 1 = Started, 2 = In Progress, 3 = Completed |
| Low Voltage Transfer | DV | The minimum line voltage allowed before the PS system transfers to battery backup. (Default units in RMS Volts) |
| High Voltage Transfer | DV | The maximum line voltage allowed before the PS system transfers to battery backup. (Default units in RMS Volts) |
| Delay Before Reboot | DV | Writing this value immediately shuts down (i.e., turns off) the output for a period equal to the indicated number of seconds, after which time the output is started. If the number of seconds required to perform the request is greater than the requested duration, then the requested shutdown and startup cycle shall be performed in the minimum time possible, but in no case shall this require more than the requested duration plus 60 seconds. If the startup should occur during a utility failure, the startup shall not occur until the utility power is restored. When read, returns the number of seconds remaining in the countdown, or –1 if no countdown is in progress. |
| Delay Before Startup | DV | Writing this value starts the output after the indicated number of seconds. Sending this command with 0 causes the startup to occur immediately. Sending this command with –1 aborts the countdown. If the output is already on, at the time the countdown reaches 0, nothing happens. On some systems, if the driver on the device side is restarted while a startup countdown is in effect, the countdown is aborted. If the countdown expires during a utility failure, the startup shall not occur until the utility power is restored. Writing this value overrides the effect of any 'Delay Before Startup' countdown or 'Delay Before Reboot' countdown in progress. When read, returns the number of seconds remaining in the countdown, or –1 if no countdown is in progress. |
| Delay Before Shutdown | DV | Writing this value shuts down (i.e., turns off) either the output after the indicated number of seconds, or sooner if the batteries become depleted. Sending this command with 0 causes the shutdown to occur immediately. Sending this command with –1 aborts the countdown. If the system is already in the desired state at the time the countdown reaches 0, there is no additional action (i.e. there is no additional action if the output is already off). On some systems, if the driver on the device side is restarted while a shutdown countdown is in effect, the countdown may be aborted. Writing this value overrides any countdown already in effect. When read, will return the number of seconds remaining until shutdown, or –1 if no shutdown countdown is in effect. |
| Test | DV | Test request/result value. **Write Value:** 0 = No test, 1 = Quick test, 2 = Deep test, 3 = Abort test. **Read Value:** 1 = Done and Passed, 2 = Done and Warning, 3 = Done and Error, 4 = Aborted, 5 = In progress, 6 = No test initiated |
| Module Reset | DV | Module Reset request value. **Write Value:** 0 = No Reset, 1 = Reset Module, 2 = Reset Module's Alarms, 3 = Reset Module's Counters. **Read Value:** Module Reset result value |
| Audible Alarm Control | DV | Read or Write Value: 1 = Disabled (Never sound), 2 = Enabled (Sound when an alarm is present), 3 = Muted (Temporarily silence the alarm). This is the requested state (Write value) or the present state (Read value) of the audible alarm. The Muted state (3) persists until the alarm would normally stop sounding. At the end of this period the value reverts to Enabled (2). Writing the value Muted (3) when the audible alarm is not sounding is accepted but otherwise has no effect. |

### 29.8 Power Generic Status

| Usage Name | Usage Type | Description |
|---|---|---|
| Present | DF | Present (1) / Not Present (0) |
| Good | DF | Good (1) / Bad (0) |
| Internal Failure | DF | Failed (1) / Not Failed (0) |
| Voltage Out Of Range | DF | Out Of Range (1) / In Range (0) |
| Frequency Out Of Range | DF | Out Of Range (1) / In Range (0) |
| Overload | DF | Overloaded (1) / Not Overloaded (0) |
| Over Charged | DF | Overcharged (1) / Not Overcharged (0) |
| Over Temperature | DF | Over Temperature (1) / Not Over Temperature (0) |
| Shutdown Requested | DF | Requested (1) / Not Requested (0) |
| Shutdown Imminent | DF | Imminent (1) / Not Imminent (0) |
| Switch On/Off | DF | On (1) indicates the switch is closed. Off (0) indicates the switch is opened. The status could be On (1) but the load still not powered if the input source power is not present. The controls associated with this status could be used to connect or disconnect Input or Output from Flow or any module or sub-module. |
| Switchable | DF | Switchable (1) / Not Switchable (0) |
| Used | DF | Used (1) / Unused (0). The status indicates this Input is presently used in the module (e.g., the Power Converter converts or transfers this Input into Output(s)). |
| Boost | DF | Boosted (1) / Not Boosted (0). The status indicates this Input is used in the module but voltage is increased to fit within nominal range values. |
| Buck | DF | Bucked (1) / Not Bucked (0). The status indicates this Input is used in the module but voltage is reduced to fit within nominal range values. |
| Initialized | DF | Initialized (1) / Not Initialized (0) |
| Tested | DF | Tested (1) / Not Tested (0) |
| Awaiting Power | DF | Awaiting Power (1) / Not Awaiting Power (0). The status indicates that the device, module, or sub-module is awaiting power from any available input source. |
| Communication Lost | DF | Communication is lost (1) / Communication is not lost (0). The status indicates that the USB agent of the device, module, or sub-module is not able to communicate with the corresponding control part of the device, module, or sub-module. As a consequence, all of the related data are no longer reliable and will not be updated until communication is reestablished. |

### 29.9 Power Device Identification

| Usage Name | Usage Type | Description |
|---|---|---|
| iManufacturer | SV | Index of a string descriptor describing the manufacturer |
| iProduct | SV | Index of a string descriptor describing the product |
| iSerialNumber | SV | Index of a string descriptor describing the device's serial number |

---

## 30. Battery System Page (0x85)

| Usage ID | Usage Name | Usage Types | Section |
|---|---|---|---|
| 00 | *Undefined* |  |  |
| 01 | **Smart Battery Battery Mode** | CL | 30.2 |
| 02 | **Smart Battery Battery Status** | NAry | 30.3.1 |
| 03 | **Smart Battery Alarm Warning** | NAry | 30.3.2 |
| 04 | **Smart Battery Charger Mode** | CL | 30.6 |
| 05 | **Smart Battery Charger Status** | CL | 30.7 |
| 06 | **Smart Battery Charger Spec Info** | CL | 30.8 |
| 07 | **Smart Battery Selector State** | CL | 30.1.1 |
| 08 | **Smart Battery Selector Presets** | CL | 30.1.2 |
| 09 | **Smart Battery Selector Info** | CL | 30.1.3 |
| 0A–0F | *Reserved* |  |  |
| 10 | Optional Mfg Function 1 | DV | 30.1 |
| 11 | Optional Mfg Function 2 | DV | 30.1 |
| 12 | Optional Mfg Function 3 | DV | 30.1 |
| 13 | Optional Mfg Function 4 | DV | 30.1 |
| 14 | Optional Mfg Function 5 | DV | 30.1 |
| 15 | Connection To SM Bus | DF | 30.1.1 |
| 16 | Output Connection | DF | 30.1.1 |
| 17 | Charger Connection | DF | 30.1.1 |
| 18 | Battery Insertion | DF | 30.1.1 |
| 19 | Use Next | DF | 30.1.2 |
| 1A | OK To Use | DF | 30.1.2 |
| 1B | Battery Supported | DF | 30.1.3 |
| 1C | Selector Revision | DF | 30.1.3 |
| 1D | Charging Indicator | DF | 30.1.3 |
| 1E–27 | *Reserved* |  |  |
| 28 | Manufacturer Access | DV | 30.2 |
| 29 | Remaining Capacity Limit | DV | 30.2 |
| 2A | Remaining Time Limit | DV | 30.2 |
| 2B | At Rate | DV | 30.2 |
| 2C | Capacity Mode | DV | 30.2 |
| 2D | Broadcast To Charger | DV | 30.2 |
| 2E | Primary Battery | DV | 30.2 |
| 2F | Charge Controller | DV | 30.2 |
| 30–3F | *Reserved* |  |  |
| 40 | Terminate Charge | Sel | 30.3.2 |
| 41 | Terminate Discharge | Sel | 30.3.2 |
| 42 | Below Remaining Capacity Limit | Sel | 30.3.2 |
| 43 | Remaining Time Limit Expired | Sel | 30.3.2 |
| 44 | Charging | Sel | 30.3.1 |
| 45 | Discharging | Sel | 30.3.1 |
| 46 | Fully Charged | Sel | 30.3.1 |
| 47 | Fully Discharged | Sel | 30.3.1 |
| 48 | Conditioning Flag | DF | 30.3 |
| 49 | At Rate OK | DF | 30.3 |
| 4A | Smart Battery Error Code | DV | 30.3 |
| 4B | Need Replacement | DF | 30.3 |
| 4C–5F | *Reserved* |  |  |
| 60 | At Rate Time To Full | DV | 30.4 |
| 61 | At Rate Time To Empty | DV | 30.4 |
| 62 | Average Current | DV | 30.4 |
| 63 | Max Error | DV | 30.4 |
| 64 | Relative State Of Charge | DV | 30.4 |
| 65 | Absolute State Of Charge | DV | 30.4 |
| 66 | Remaining Capacity | DV | 30.4 |
| 67 | Full Charge Capacity | DV | 30.4 |
| 68 | Run Time To Empty | DV | 30.4 |
| 69 | Average Time To Empty | DV | 30.4 |
| 6A | Average Time To Full | DV | 30.4 |
| 6B | Cycle Count | DV | 30.4 |
| 6C–7F | *Reserved* |  |  |
| 80 | Battery Pack Model Level | SV | 30.5 |
| 81 | Internal Charge Controller | SF | 30.5 |
| 82 | Primary Battery Support | SF | 30.5 |
| 83 | Design Capacity | SV | 30.5 |
| 84 | Specification Info | SV | 30.5 |
| 85 | Manufacture Date | SV | 30.5 |
| 86 | Serial Number | SV | 30.5 |
| 87 | iManufacturer Name | SV | 30.5 |
| 88 | iDevice Name | SV | 30.5 |
| 89 | iDevice Chemistry | SV | 30.5 |
| 8A | Manufacturer Data | SV | 30.5 |
| 8B | Rechargable | SV | 30.5 |
| 8C | Warning Capacity Limit | SV | 30.5 |
| 8D | Capacity Granularity 1 | SV | 30.5 |
| 8E | Capacity Granularity 2 | SV | 30.5 |
| 8F | iOEM Information | SV | 30.5 |
| 90–BF | *Reserved* |  |  |
| C0 | Inhibit Charge | DF | 30.6 |
| C1 | Enable Polling | DF | 30.6 |
| C2 | Reset To Zero | DF | 30.6 |
| C3–CF | *Reserved* |  |  |
| D0 | AC Present | DV | 30.7 |
| D1 | Battery Present | DV | 30.7 |
| D2 | Power Fail | DV | 30.7 |
| D3 | Alarm Inhibited | DV | 30.7 |
| D4 | Thermistor Under Range | DV | 30.7 |
| D5 | Thermistor Hot | DV | 30.7 |
| D6 | Thermistor Cold | DV | 30.7 |
| D7 | Thermistor Over Range | DV | 30.7 |
| D8 | Voltage Out Of Range | DV | 30.7 |
| D9 | Current Out Of Range | DV | 30.7 |
| DA | Current Not Regulated | DV | 30.7 |
| DB | Voltage Not Regulated | DV | 30.7 |
| DC | Master Mode | DV | 30.7 |
| DD–EF | *Reserved* |  |  |
| F0 | Charger Selector Support | SF | 30.8 |
| F1 | Charger Spec | SV | 30.8 |
| F2 | Level 2 | SF | 30.8 |
| F3 | Level 3 | SF | 30.8 |
| F4–FFFF | *Reserved* |  |  |

*Table 30.1: Battery System Page*

### 30.1 Battery System Settings and Controls

| Usage Name | Usage Type | Description |
|---|---|---|
| Optional Mfg Function 1 | DV | Manufacturer-specific function |
| Optional Mfg Function 2 | DV | Manufacturer-specific function |
| Optional Mfg Function 3 | DV | Manufacturer-specific function |
| Optional Mfg Function 4 | DV | Manufacturer-specific function |
| Optional Mfg Function 5 | DV | Manufacturer-specific function |

#### 30.1.1 Selector State

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Selector State** | CL |  |
| Connection To SM Bus | DF | State of connection to the system SMBus |
| Output Connection | DF | Id of the connected Output to the specified battery |
| Charger Connection | DF | Id of the specified Charger to the specified Battery |
| Battery Insertion | DF | Insertion status of the specified Battery into the system |

#### 30.1.2 Selector Presets

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Selector Presets** | CL |  |
| Use Next | DF | Whether or not this Battery will be used for next discharge |
| OK To Use | DF | Whether or not this Battery is usable |

#### 30.1.3 Selector Info

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Selector Info** | CL |  |
| Battery Supported | DF | Whether or not this Battery is supported by the selector |
| Selector Revision | DF | Version of the Smart Battery Selector specification. For revision 1.0, the value will be 001 |
| Charging Indicator | DF | Indicates whether the selector reports the charger's status in the POWERBY nibble of SelectorState |

### 30.2 Battery Controls

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Battery Mode** | CL |  |
| Manufacturer Access | DV | Meaning is according to the Smart Battery Data Specification |
| Remaining Capacity Limit | DV | Sets the value of the battery's remaining capacity, which causes a Remaining Capacity alarm to be sent. Whenever the battery's remaining capacity falls below the value in the RemainingCapacity alarm register, the battery periodically issues a RemainingCapacity alarm. (Units are defined by CapacityMode.) |
| Remaining Time Limit | DV | Sets the value of the battery's remaining time, which causes the RemainingTimeLimit control to be activated. Whenever the battery's remaining time falls below the value in the RemainingTimeLimit register, the battery periodically issues a RemainingTimeLimitExpired alarm. (Units are seconds.) |
| At Rate | DV | Sets the value used by the battery to calculate 'At Rate Time To Full', 'At Rate Time To Empty' or 'AT Rate OK'. ('At Rate' units are defined by 'Capacity Mode'.) |
| Capacity Mode | DV | Battery capacity units are as follows: 0 = maH (used in SMB), 1 = mwH (used in SMB), 2 = %, 3 = Boolean support only (OK or failed) |
| Broadcast To Charger | DV | Enables or disables broadcast to charger |
| Primary Battery | DV | Whether operating in primary or secondary mode |
| Charge Controller | DV | Whether internal charge control is enabled |

### 30.3 Battery Status

| Usage Name | Usage Type | Description |
|---|---|---|
| Conditioning Flag | DF | Whether conditioning cycle needed (else Battery is OK) |
| At Rate OK | DF | After an AtRate value setting, the device sets AtRateOK to 0 and calculates the AtRateTimeToFull and AtRateToEmpty values. When these values are already available, the device sets AtRateOK to 1. |
| Smart Battery Error Code | DV | An Smart Battery-specific 4-bit error code |
| Need Replacement | DF | Whether the battery needs replacement |

#### 30.3.1 Status

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Battery Status** | NAry |  |
| Charging | Sel | Battery is charging |
| Discharging | Sel | Battery is discharging |
| Fully Charged | Sel | Battery is fully-charged |
| Fully Discharged | Sel | Battery is fully discharged |

#### 30.3.2 Alarm

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Alarm Warning** | NAry |  |
| Terminate Charge | Sel | Terminates charge |
| Terminate Discharge | Sel | Terminates discharge |
| Below Remaining Capacity Limit | Sel | Is below |
| Remaining Time Limit Expired | Sel | Has expired |

### 30.4 Battery Measures

| Usage Name | Usage Type | Description |
|---|---|---|
| At Rate Time To Full | DV | The predicted remaining time to fully charge the battery at the AtRate value. (Units are minutes.) |
| At Rate Time To Empty | DV | The predicted operating time if the battery is discharged at the AtRate value. |
| Average Current | DV | A one-minute rolling average of the current being supplied or accepted through the battery terminals |
| Max Error | DV | The expected margin error (%) in the state of charge calculation |
| Relative State Of Charge | DV | The predicted remaining battery capacity expressed as a percentage of the last measured full charge capacity. (Units are %.) |
| Absolute State Of Charge | DV | The predicted remaining battery capacity expressed as a percentage of design capacity. (Units are %. The value may be greater than 100%.) |
| Remaining Capacity | DV | The predicted remaining capacity. (See CapacityMode for units.) |
| Full Charge Capacity | DV | The predicted pack capacity when it is fully charged. (See CapacityMode for units.) |
| Run Time To Empty | DV | The predicted remaining battery life, in minutes, at the present rate of discharge. The RunTimeToEmpty is calculated based on either current or power depending on the CapacityMode setting |
| Average Time To Empty | DV | A one-minute rolling average, in minutes, of the predicted remaining battery time life. The AverageTimeToEmpty is calculated based on either current or power depending on the CapacityMode setting |
| Average Time To Full | DV | A one-minute rolling average, in minutes, of the predicted remaining time until the battery reaches full charge |
| Cycle Count | DV | The number, in cycles, of charge/discharge cycles the battery has experienced. |

### 30.5 Battery Settings

| Usage Name | Usage Type | Description |
|---|---|---|
| Battery Pack Model Level | SV | Battery model level for the battery pack: 0 = Basic Model, 1 = Intelligent Model, 2 = Smart Battery |
| Internal Charge Controller | SF | Whether charge controller function supported in the battery pack |
| Primary Battery Support | SF | Whether Primary battery function supported in the battery pack |
| Design Capacity | SV | The theoretical capacity of a new pack. (See CapacityMode for units.) |
| Specification Info | SV | The version number of the Smart Battery Data Specification. |
| Manufacture Date | SV | The date the pack was manufactured in a packed integer. The date is packed in the following fashion: `(year - 1980) * 512 + month * 32 + day` |
| Serial Number | SV | The cell pack serial number |
| iManufacturer Name | SV | Index of a string descriptor containing the battery manufacturer's name |
| iDevice Name | SV | Index of a string descriptor containing the battery's name |
| iDevice Chemistry | SV | Index of a string descriptor containing the battery's chemistry |
| Manufacturer Data | SV | A binary data block containing manufacturer specific data |
| Rechargable | SV | Whether the battery is rechargable |
| Warning Capacity Limit | SV | OEM-designed battery warning capacity. (Units are defined by CapacityMode.) |
| Capacity Granularity 1 | SV | Battery capacity granularity between low and warning. (Units are defined by CapacityMode.) |
| Capacity Granularity 2 | SV | Battery capacity granularity between warning and full. (Units are defined by CapacityMode) |
| iOEM Information | SV | Index of a string descriptor defining OEM specific information for the battery |

### 30.6 Charger Controls

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Charger Mode** | CL |  |
| Inhibit Charge | DF | Inhibit or enable charging |
| Enable Polling | DF | Enable or disable polling |
| Reset To Zero | DF | Reset Charging Current and Voltage values to zero |

### 30.7 Charger Status

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Charger Status** | CL |  |
| AC Present | DF | Present / Not Present |
| Battery Present | DF | Present / Not Present |
| Power Fail | DF | Low / Not Low |
| Alarm Inhibited | DF | Inhibited / Not Inhibited |
| Thermistor Under Range | DF | Under / Not Under |
| Thermistor Hot | DF | Hot / Not Hot |
| Thermistor Cold | DF | Cold / Not Cold |
| Thermistor Over Range | DF | Over / Not Over |
| Voltage Out Of Range | DF | Not Valid / Valid |
| Current Out Of Range | DF | Not Valid / Valid |
| Current Not Regulated | DF | Not Regulated / Regulated |
| Voltage Not Regulated | DF | Not Regulated / Regulated |
| Master Mode | DF | Master Mode (polling is enabled) / Slave Mode (polling is disabled) |

### 30.8 Charger Settings

| Usage Name | Usage Type | Description |
|---|---|---|
| **Smart Battery Charger Spec Info** | CL |  |
| Charger Selector Support | SF | Yes / No |
| Charger Spec | SV | Specification reference. (0001 for SMB charger 1.0) |
| Level 2 | SF | Charger at level 2. (Level 1 default) |
| Level 3 | SF | Charger at level 3. (Level 1 default) |
