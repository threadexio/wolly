{ self
, ...
}:

{ config
, pkgs
, lib
, ...
}:

with lib;

let
  cfg = config.services.wolly;
in

{
  options.services.wolly = {
    enable = mkEnableOption "wolly";

    package = mkOption {
      description = ''
        The wolly package to use.
      '';
      type = types.package;
      default = self.packages.${pkgs.stdenv.system}.default;
    };

    upstream = mkOption {
      type = types.nonEmptyListOf (
        types.submodule {
          options = {
            address = mkOption {
              description = ''
                Address of the upstream host.
              '';
              type = types.str;
              example = "192.168.1.42";
            };

            mac = mkOption {
              description = ''
                MAC address of the upstream host.
              '';
              type = types.str;
              example = "12:34:56:78:9a:bc";
            };

            brd = mkOption {
              type = types.str;
              description = ''
                Broadcast address of the host.
              '';
              example = "192.168.1.255";
            };
          };
        }
      );

      default = [ ];
    };

    forward = mkOption {
      type = types.nonEmptyListOf (
        types.submodule {
          options = {
            from = mkOption {
              description = ''
                Forward connections from this endpoint.
              '';
              type = types.str;
              example = "0.0.0.0:8000";
            };

            to = mkOption {
              description = ''
                Forward connections to this endpoint.

                The host provided here must have an accompanying entry in `upsteam`.
              '';
              type = types.str;
              example = "192.168.1.42:15000";
            };

            wait-for = mkOption {
              description = ''
                Instruct wolly to wait this many seconds after sending the WoL packet.

                This can be used to give time to the target host to come up.
              '';
              type = types.ints.unsigned;
              default = 0;
              example = 5;
            };

            max-attempts = mkOption {
              description = ''
                After sending the WoL packet, wolly will try to connect to the target host this many times before giving up.
              '';
              type = types.ints.positive;
              default = 5;
              example = 10;
            };

            retry-delay = mkOption {
              description = ''
                wolly will wait this many seconds before retrying to connect to the target host.
              '';
              type = types.ints.unsigned;
              default = 1;
              example = 5;
            };

            retry-factor = mkOption {
              description = ''
                With each failed connection to the target host, the retry delay will grow this much.

                The following formula gives the retry delay:

                ```
                delay = retry-delay × retry-factor ^ attempt
                ```
              '';
              type = types.float;
              default = 2.0;
              example = 1.5;
            };
          };
        }
      );

      default = [ ];
    };

    extraConfig = mkOption {
      description = ''
        Extra config options for wolly.
      '';
      type = types.lines;
      default = "";
      example = ''
        forward :10000-10100 to 10.0.0.42:30000-30100
      '';
    };
  };

  config = mkIf cfg.enable {
    nixpkgs.overlays = [ self.overlays.default ];

    users.users.wolly = {
      description = "wolly service user";
      home = "/var/empty";
      isSystemUser = true;
      group = "wolly";
    };
    users.groups.wolly = { };

    systemd.services.wolly = {
      description = "Transparent TCP Wake-On-Lan proxy";
      after = [ "network.target" ];
      wantedBy = [
        "network.target"
        "multi-user.target"
      ];

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/wolly /etc/wolly.conf";
        AmbientCapabilities = [
          "CAP_NET_BIND_SERVICE"
          "CAP_NET_BROADCAST"
        ];
        RemoveIPC = true;
        User = "wolly";
        UMask = 0077;
        NoNewPrivileges = true;
        MemoryDenyWriteExecute = true;
        LockPersonality = true;
        CapabilityBoundingSet = [
          "~CAP_SYS_TIME"
          "~CAP_SYS_PACCT"
          "~CAP_KILL"
          "~CAP_(DAC_*|FOWNER|IPC_OWNER)"
          "~CAP_LINUX_IMMUTABLE"
          "~CAP_IPC_LOCK"
          "~CAP_BPF"
          "~CAP_SYS_TTY_CONFIG"
          "~CAP_SYS_BOOT"
          "~CAP_SYS_CHROOT"
          "~CAP_(CHOWN|FSETID|SETFCAP)"
          "~CAP_SET(UID|GID|PCAP)"
          "~CAP_MAX_*"
          "~CAP_SYS_PTRACE"
          "~CAP_SYS_(NICE|RESOURCE)"
          "~CAP_NET_ADMIN"
          "CAP_NET_(BIND_SERVICE|BROADCAST)"
          "~CAP_NET_RAW"
          "~CAP_AUDIT_*"
          "~CAP_SYS_ADMIN"
        ];
        ProtectHostname = true;
        ProtectKernelTunables = true;
        ProtectSystem = "strict";
        ProtectProc = "invisible";
        ProcSubset = "pid";
        ProtectHome = "yes";
        ProtectClock = "yes";
        ProtectKernelLogs = "yes";
        ProtectControlGroups = "yes";
        ProtectKernelModules = "yes";
        PrivateUsers = true;
        PrivateTmp = true;
        PrivateDevices = true;
        DeviceAllow = [ ];
        RestrictNamespaces = [
          "~user"
          "~pid"
          "~net"
          "~uts"
          "~mnt"
          "~cgroup"
          "~ipc"
        ];
        SystemCallFilter = [
          "~@cpu-emulation"
          "~@debug"
          "~@module"
          "~@mount"
          "~@obsolete"
          "~@privileged"
          "~@reboot"
          "~@resources"
          "~@swap"
        ];
      };
    };

    environment.etc."wolly.conf".text =
      let
        upstreamToConfig =
          { address
          , mac
          , brd
          ,
          }:
          "upstream ${address} mac ${mac} brd ${brd}";

        forwardToConfig =
          { from
          , to
          , wait-for
          , max-attempts
          , retry-delay
          , retry-factor
          ,
          }:
          "forward ${from} to ${to} wait-for ${toString wait-for} max-attempts ${toString max-attempts} retry-delay ${toString retry-delay} retry-factor ${toString retry-factor}";
      in
      ''
        # Generated by the NixOS module services.wolly
        # DO not edit manually!
      ''
      + "\n"
      + (lib.concatLines (map upstreamToConfig cfg.upstream))
      + (lib.concatLines (map forwardToConfig cfg.forward))
      + "\n"
      + cfg.extraConfig;
  };
}
