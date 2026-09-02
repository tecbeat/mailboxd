//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


import React from "react";
import { ExternalLink } from "lucide-react";

interface SourceLinkButtonProps {
  href?: string;
  size?: number;
  title?: string;
}

// AGPL §13 — link to the corresponding source of this modified version.
const SourceLinkButton: React.FC<SourceLinkButtonProps> = ({
  href = "https://git.teccave.de/tecbeat/mailboxd",
  size = 18,
  title = "Source code",
}) => {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      title={title}
      className="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors text-xs font-medium"
    >
      <ExternalLink style={{ width: size, height: size }} />
      <span>Source</span>
    </a>
  );
};

export const GithubLinkButton = SourceLinkButton;