[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=plastic&logo=rust&logoColor=white)](https://www.youtube.com/watch?v=cE0wfjsybIQ&t=73s)
[![License](https://img.shields.io/github/license/4lve/SteelMC?style=social)](https://github.com/4lve/SteelMC/blob/master/LICENSE) 
[![SteelMC](https://dcbadge.limes.pink/api/server/MwChEHnAbh?style=social)](https://discord.gg/MwChEHnAbh) 
![Tests](https://github.com/4lve/SteelMC/actions/workflows/test.yml/badge.svg) 
![Lint](https://github.com/4lve/SteelMC/actions/workflows/lint.yml/badge.svg)
![Build](https://github.com/4lve/SteelMC/actions/workflows/release.yml/badge.svg)



<div align="center">

# Steel

   <p align="center" width="66%">
     <img src="https://i.imgur.com/lFQ6jH2.png" alt="Logo" width="66%">
   </p>

Steel is a lightweight Rust implementation of the Minecraft server, partially based on [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin).  
It focuses on clean code, performance, extensibility, and ease of use.
</div>

---

## 🔗 Links
<div align="center">
   
[Discord](https://discord.gg/MwChEHnAbh) | [Parchment Mappings Tutorial](https://parchmentmc.org/docs/getting-started) | [GitCraft](https://github.com/WinPlay02/GitCraft)
</div>
   
---

## ⚙ How to Contribute

1. Identify a feature you'd like to add or an issue to work on.
   You should always create a post in the channel [feature-discussion](https://canary.discord.com/channels/1428487339759370322/1429074039015473272) when considering adding a major feature.
2. Decompile the latest version of Minecraft using Parchment mappings.  
   *(the `master` branch currently targets Minecraft **1.21.10**).*

   Alternatively, you may use [GitCraft](https://github.com/WinPlay02/GitCraft) for this task.

   If you choose to use GitCraft, run the command 
   ```bash
   ./gradlew run --args="--mappings=mojmap_parchment --only-stable"
   ```
   in the GitCraft directory and keep in mind that you *may* have to implement this [change](https://github.com/WinPlay02/GitCraft/pull/29).
3. Fork the `master` branch of this repository.  
4. Examine the vanilla implementation and translate it into idiomatic Rust as cleanly and efficiently as possible.  
5. Commit your changes to your fork and open a pull request.

> [!NOTE]
> It is highly recommended to join the [Discord server](https://discord.gg/MwChEHnAbh) and reach out to [4lve](https://github.com/4lve) if you have code-related questions or encounter any ambiguities.

> [!IMPORTANT]
> This project is still in a very early stage of development.
