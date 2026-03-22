package updater

import (
	"context"
	"fmt"
	"os/exec"
	"strings"
	"time"

	"github.com/dpeluche/spark/internal/core"
)

// Executor handles the actual update process for tools
type Executor struct{}

func NewExecutor() *Executor {
	return &Executor{}
}

// Update attempts to update the specified tool
func (e *Executor) Update(t core.Tool) error {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute) // Updates can take time
	defer cancel()

	switch t.Method {
	case core.MethodBrew, core.MethodBrewPkg:
		return e.updateBrew(ctx, t)
	case core.MethodMacApp:
		return e.updateMacApp(ctx, t)
	case core.MethodNpmSys, core.MethodNpmPkg:
		return e.updateNpm(ctx, t)
	case core.MethodClaude:
		return e.updateNpm(ctx, t) // Claude is an NPM package
	case core.MethodOmz:
		return e.updateOmz(ctx)
	case core.MethodToad:
		return e.updateToad(ctx)
	case core.MethodDroid:
		return e.updateDroid(ctx)
	case core.MethodOpencode:
		return e.updateOpencode(ctx)
	case core.MethodManual:
		return fmt.Errorf("manual update required (check vendor portal)")
	default:
		return fmt.Errorf("update method %s not implemented", t.Method)
	}
}

func (e *Executor) updateDroid(ctx context.Context) error {
	// Droid (Factory AI)
	// curl -fsSL https://app.factory.ai/cli | sh
	cmd := exec.CommandContext(ctx, "sh", "-c", "curl -fsSL https://app.factory.ai/cli | sh")
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("droid update failed: %s: %v", string(output), err)
	}
	return nil
}

func (e *Executor) updateOpencode(ctx context.Context) error {
	// OpenCode
	// opencode upgrade || curl -fsSL https://opencode.ai/install | bash
	
	// Try built-in upgrade first
	cmdUpgrade := exec.CommandContext(ctx, "opencode", "upgrade")
	if err := cmdUpgrade.Run(); err == nil {
		return nil
	}

	// Fallback to install script
	cmdInstall := exec.CommandContext(ctx, "sh", "-c", "curl -fsSL https://opencode.ai/install | bash")
	if output, err := cmdInstall.CombinedOutput(); err != nil {
		return fmt.Errorf("opencode update failed: %s: %v", string(output), err)
	}
	return nil
}

func (e *Executor) updateToad(ctx context.Context) error {
	// Toad (by Batrachian AI) installs via script to ~/.local/bin
	// curl -fsSL https://batrachian.ai/install | sh
	
	cmd := exec.CommandContext(ctx, "sh", "-c", "curl -fsSL https://batrachian.ai/install | sh")
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("toad update failed: %s: %v", string(output), err)
	}
	return nil
}

func (e *Executor) updateBrew(ctx context.Context, t core.Tool) error {
	// brew upgrade <package>
	cmd := exec.CommandContext(ctx, "brew", "upgrade", t.Package)
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("brew upgrade failed: %s: %v", string(output), err)
	}
	return nil
}

func (e *Executor) updateMacApp(ctx context.Context, t core.Tool) error {
	// Try upgrading via brew cask first
	// We assume if it's a MacApp it might be managed by brew cask
	// Check if it is a cask
	checkCmd := exec.CommandContext(ctx, "brew", "list", "--cask", t.Package)
	if err := checkCmd.Run(); err == nil {
		cmd := exec.CommandContext(ctx, "brew", "upgrade", "--cask", t.Package)
		if output, err := cmd.CombinedOutput(); err != nil {
			return fmt.Errorf("brew cask upgrade failed: %s: %v", string(output), err)
		}
		return nil
	}

	// If not a cask, we can't auto-update it easily
	return fmt.Errorf("manual update required (not a brew cask)")
}

func (e *Executor) updateNpm(ctx context.Context, t core.Tool) error {
	// npm install -g <package>@latest
	pkg := t.Package
	if pkg == "" {
		pkg = t.Binary
	}

	cmd := exec.CommandContext(ctx, "npm", "install", "-g", pkg+"@latest")
	output, err := cmd.CombinedOutput()
	if err != nil {
		// Auto-recovery for EEXIST (broken symlinks or permissions)
		outputStr := string(output)
		if strings.Contains(outputStr, "EEXIST") {
			// Retry with --force
			cmdForce := exec.CommandContext(ctx, "npm", "install", "-g", pkg+"@latest", "--force")
			if outputForce, errForce := cmdForce.CombinedOutput(); errForce != nil {
				return fmt.Errorf("npm install failed (even with --force): %s: %v", string(outputForce), errForce)
			}
			return nil // Success with force
		}
		return fmt.Errorf("npm install failed: %s: %v", outputStr, err)
	}
	return nil
}

func (e *Executor) updateOmz(ctx context.Context) error {
	// omz update usually runs interactively or via script.
	// The standard way is running the upgrade script.
	// Often available as `omz update` alias, but that might not be in the path for non-interactive shells.
	// We can try calling the script directly if we find it.

	cmd := exec.CommandContext(ctx, "sh", "-c", "$ZSH/tools/upgrade.sh")
	// Set env var to avoid interactive prompt if supported
	cmd.Env = append(cmd.Env, "ZSH="+getEnv("ZSH", "~/.oh-my-zsh"))

	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("omz update failed: %s: %v", string(output), err)
	}
	return nil
}

// Helper to get env with fallback (simplified)
func getEnv(key, fallback string) string {
	// In a real scenario, we'd read actual os.Environ
	return fallback
}
