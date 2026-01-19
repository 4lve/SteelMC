# How to decompile minecraft

before the 1.21.11 it was obviouscated code so mapping was needed, but this servers target is >=1.21.11 this
documentation will only help you with these version.

## Requirements

Below there is a command for 1.21.11 and for 26.1 snapshot 3 but you can use any version with it.
Use the minecraft launcher (also Prism, etc) create an instance with your target version, next start it this will
download the jar file. This is located depending on the launcher and OS but then you have the jar file you can run one
of the options below.

## There are two ways

- view the minecraft source code you can do is clone [gitcraft](https://github.com/WinPlay02/GitCraft), and run
  `./gradlew run --args="--only-stable --min-version=1.21.11 --only-unobfuscated"` it will create
  `minecraft-repo-mojmap-unobfuscated-min-1.21.11-stable/minecraft` with all source code

- or you can download the [vinflower.jar](https://github.com/Vineflower/vineflower/releases) and then run this command
  ``java -jar vineflower-1.11.2.jar ./minecraft-26.1-snapshot-3-client.jar --folder minecraft_26.1``. That will create a
  folder named `minecraft_26.1`

## Other useful resources
- videos on YouTube which will give you also a good tutorial