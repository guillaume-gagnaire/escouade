# L’app mobile

Escouade sur ton téléphone, pour Android et iPhone : ce qui attend ta réponse sur ton poste et tes serveurs, dans un seul fil, et de quoi débloquer un agent en quelques secondes, sans cloud entre vous.

![Le fil « À toi » : les agents arrêtés, puis la boîte](/images/mobile-thread.jpg)

## Ce qu’elle fait

- **Ce qui t’attend, partout.** Les agents arrêtés en tête (une autorisation, une question, un plan), puis la boîte « À toi » (un choix entre des rendus, un feu vert, une réponse, un ticket à valider). Chaque demande dit son espace, son projet et son agent.
- **Répondre en sachant ce que tu acceptes.** Une commande se lit en entier avant d’être autorisée. Chaque réponse qui fait agir un agent passe par le verrou du téléphone : l’empreinte, Face ID ou le code. « Toujours autoriser » ne se coche jamais depuis le téléphone.
- **Parler à tes agents.** Un message tapé ou dicté, avec une photo ; leur conversation ; le Kanban d’un projet, où valider un ticket « À tester » ou le renvoyer.
- **Créer.** Un agent et son premier message, un ticket dicté, un projet sur un serveur.
- **Être prévenu.** Une notification quand un agent t’attend, chiffrée de bout en bout : le relais ne voit qu’un réveil, jamais le texte.

![Une autorisation dépliée, la commande en entier](/images/mobile-authorization.jpg)

## Comment elle se relie

Le téléphone s’appaire à ton poste ou à un serveur Escouade par un QR code, puis lui parle par un tunnel WireGuard chiffré, directement. Aucun compte Escouade, aucune donnée chez nous : les clés restent dans le Keystore ou le Trousseau du téléphone, et un téléphone perdu se révoque depuis le poste. La [documentation de l’app](/docs/mobile-app/) détaille l’appairage, les espaces et le verrou.

## L’obtenir

L’app arrive d’abord en test fermé sur Google Play et dans TestFlight, puis sur les deux stores. Elle demande Escouade 2.3 ou plus récent sur le poste ou le serveur. Ce qu’elle envoie, et à qui, est dans sa [politique de confidentialité](/mobile/privacy/).
