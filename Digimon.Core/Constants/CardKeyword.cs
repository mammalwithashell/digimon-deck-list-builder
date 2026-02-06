using System.ComponentModel;

namespace Digimon.Core.Constants
{
    public enum CardKeyword
    {
        Blocker,
        Rush,
        Piercing,
        Recovery,
        [Description("Security Attack")]
        SecurityAttack,
        Draw,
        Memory,
        [Description("De-Digivolve")]
        DeDigivolve,
        [Description("Digi-Burst")]
        DigiBurst,
        Delay,
        Decoy,
        Retaliation,
        [Description("Armor Purge")]
        ArmorPurge,
        Jamming,
        Digisorption,
        Reboot,
        Blitz,
        Save,
        [Description("Material Save")]
        MaterialSave,
        Evade,
        Raid,
        Alliance,
        Barrier,
        [Description("Blast Digivolve")]
        BlastDigivolve,
        [Description("Mind Link")]
        MindLink,
        Fortitude,
        Partition,
        Collision,
        Scapegoat,
        [Description("Blast DNA Digivolve")]
        BlastDNADigivolve,
        Vortex,
        Overclock,
        Iceclad,
        Decode,
        Fragment,
        Execute,
        Progress,
        Link,
        Training,
        [Description("Use Req.")]
        UseRequirement
    }
}
