import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PanelLeft } from "lucide-react";
import { SettingRow } from "./SettingRow";
import { Dropdown } from "./Dropdown";
import { useT } from "../i18n";

// Ledge: which screen edge the notch hugs. "top" is upstream Nectar behaviour.
export function NotchPosition() {
  const t = useT();
  const [value, setValue] = useState(() => (localStorage.getItem("nectar-notch-position") === "left" ? "left" : "top"));

  const change = (next: string) => {
    setValue(next);
    localStorage.setItem("nectar-notch-position", next);
    invoke("save_setting", { key: "nectar-notch-position", value: next }).catch(() => { });
  };

  return (
    <SettingRow icon={PanelLeft} label={t("notch.position")} desc={t("notch.position.desc")}>
      <Dropdown
        value={value}
        onChange={change}
        options={[
          { value: "top", label: t("notch.position.top") },
          { value: "left", label: t("notch.position.left") },
        ]}
      />
    </SettingRow>
  );
}
